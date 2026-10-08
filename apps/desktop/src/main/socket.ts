// The socket lifecycle: read the runtime file, verify the pid, connect, and
// retry on drop.
//
// **Split out of `connection.ts`**, which named this as the next real seam
// once seven other files had already come out of it and what was left was one
// thing — a state machine, plus the arrival handler that folds each message
// into it. This is not a slice of that: it owns the wire underneath the state
// machine and nothing about what a message means, which is why the split does
// not put the state machine's transitions in two files. Every message this
// reads is handed to the caller's own `arrived` whole and unopened; deciding
// what it means, and closing the socket on purpose because of what it meant,
// both stay in `connection.ts`.
//
// **Bridge finds Fleet through a runtime file carrying port, pid and protocol
// version, and verifies the pid before connecting** — `runtime-file.ts` is
// where that happens, and it is what tells "Fleet is not running" from
// "running and unreachable" apart, two states a bare connection timeout would
// render identical.

import WebSocket from "ws";

import { PROTOCOL_ID, speaksOurProtocol } from "@armada/protocol";
import type { Connection } from "@armada/protocol";
import { HOST, machinePath, read } from "./runtime-file";

/** How long to wait before reading the runtime file again. */
const RETRY_MS = 2000;

/**
 * How long a socket may sit without a first message before Bridge says so. A
 * Fleet that is serving answers in milliseconds, so a Bridge opened against one
 * that has been up for hours never gets past `connecting`.
 */
const ANSWER_GRACE_MS = 1500;

/**
 * How old a Fleet process may be and still be booting. **Measured once, 6 Oct
 * 2026: one to two minutes** between the runtime file and `serving`, which is
 * `reconcile` reading every Job the store holds. Five minutes leaves that room
 * twice over; a Fleet older than this that does not answer is wedged, and
 * `unreachable` is the reading that does not promise it will come.
 */
const BOOT_WINDOW_MS = 5 * 60_000;

/** The one connection state that carries which Fleet Bridge is talking to. */
export type BridgeStateFleet = Extract<Connection, { state: "connected" }>["fleet"];

export type FleetSocketWiring = {
  home: string | undefined;
  now: () => number;
  /** The two waits above, shortened where a test cannot spend minutes. */
  timing?: { graceMs: number; bootMs: number };
  /** A connection state to settle, this instant. */
  settle: (connection: Connection) => void;
  /**
   * A fresh socket is open, before anything has arrived on it — the instant
   * the next resync is this socket's first, which is what tells Fleet coming
   * back from a plain gap in a stream that never stopped.
   */
  opened: () => void;
  /** A message arrived on the open socket, unparsed. */
  arrived: (text: string, fleet: BridgeStateFleet) => void;
};

/**
 * Read the runtime file, verify the pid, connect — and keep trying, on a
 * drop or a refusal, for as long as `start()` has been called more recently
 * than `stop()`.
 */
export class FleetSocket {
  private readonly wiring: FleetSocketWiring;
  private socket: WebSocket | null = null;
  private retry: ReturnType<typeof setTimeout> | null = null;
  private unreachableSince: number | null = null;
  /** Fires when the open socket has said nothing for too long. */
  private watch: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;

  constructor(wiring: FleetSocketWiring) {
    this.wiring = wiring;
  }

  /** Read the runtime file, verify the pid, connect. That order, always. */
  start(): void {
    this.stopped = false;
    void this.attach();
  }

  stop(): void {
    this.stopped = true;
    if (this.retry !== null) clearTimeout(this.retry);
    this.retry = null;
    this.unwatch();
    this.socket?.close();
    this.socket = null;
  }

  /**
   * Close the socket on purpose, because the caller read a message it could
   * not use — one this Bridge could not parse, or a resync naming a version
   * this Bridge will not speak. The socket's own `close` event carries on
   * from there exactly as an ordinary drop would.
   */
  close(): void {
    this.socket?.close();
  }

  /**
   * The resync that follows a reconnection clears what a drop had recorded.
   * `connection.ts` calls this once the greeting it was waiting for arrives.
   */
  resetUnreachable(): void {
    this.unreachableSince = null;
  }

  private async attach(): Promise<void> {
    if (this.stopped) return;
    const path = machinePath(this.wiring.home);
    if (path === null) {
      this.wiring.settle({
        state: "runtime_file_refused",
        fault: {
          why: "unreadable",
          path: "",
          detail: "HOME is not set, so the machine directory cannot be resolved",
        },
      });
      return this.later();
    }

    const presence = await read(path);
    if (this.stopped) return;

    if (presence.at === "absent" || presence.at === "stale") {
      // Both render as "Fleet is not running", and the screen says which.
      // Neither opens a socket: a stale file's port may not be Fleet's.
      this.unreachableSince = null;
      this.wiring.settle({ state: "not_running", absence: presence.absence });
      return this.later();
    }
    if (presence.at === "refused") {
      this.unreachableSince = null;
      this.wiring.settle({ state: "runtime_file_refused", fault: presence.fault });
      return this.later();
    }

    const fleet = presence.fleet;
    // Read before connecting, so a protocol Bridge will not speak is a refusal
    // rather than a bad first message.
    if (!speaksOurProtocol(fleet.protocolId)) {
      this.wiring.settle({
        state: "protocol_mismatch",
        fleet,
        speaks: fleet.protocolId,
        expected: PROTOCOL_ID,
      });
      return this.later();
    }

    this.wiring.settle(
      this.unreachableSince === null
        ? { state: "connecting", fleet }
        : {
            state: "unreachable",
            fleet,
            detail: "the socket has not answered",
            sinceMs: this.unreachableSince,
          },
    );
    this.open(fleet.port, fleet);
  }

  private open(port: number, fleet: BridgeStateFleet): void {
    const socket = new WebSocket(`ws://${HOST}:${port}/events`);
    this.socket = socket;
    // The next resync to arrive is this socket's first, so it is Fleet coming
    // back rather than a gap in a stream that never stopped.
    this.wiring.opened();

    // **A refusal never reaches this.** Fleet binds its port before it writes
    // the runtime file and serves only after `reconcile`, so in between the
    // kernel accepts the connection and nothing answers it: no `error`, no
    // `close`, and the socket would sit in `connecting` for as long as boot
    // takes. The timer is what notices.
    this.unwatch();
    this.watch = setTimeout(() => this.unanswered(fleet), this.wiring.timing?.graceMs ?? ANSWER_GRACE_MS);

    socket.on("message", (data: WebSocket.RawData) => {
      this.unwatch();
      this.wiring.arrived(String(data), fleet);
    });
    socket.on("error", (cause: Error) => this.dropped(fleet, cause.message));
    socket.on("close", () => this.dropped(fleet, "the connection closed"));
  }

  /**
   * The socket is open and has said nothing. A Fleet process younger than the
   * boot window is starting; an older one is not answering. Either way the
   * socket stays open, so a Fleet that does come up still connects.
   */
  private unanswered(fleet: BridgeStateFleet): void {
    this.watch = null;
    if (this.socket === null || this.stopped) return;
    const bootMs = this.wiring.timing?.bootMs ?? BOOT_WINDOW_MS;
    const now = this.wiring.now();
    const born = Date.parse(fleet.startedAt);
    const age = now - born;
    // A start time that will not parse says nothing about age, so it cannot
    // earn the starting view.
    if (!Number.isNaN(born) && age < bootMs) {
      this.wiring.settle({ state: "starting", fleet });
      this.watch = setTimeout(() => this.unanswered(fleet), bootMs - age);
      return;
    }
    if (this.unreachableSince === null) this.unreachableSince = now;
    this.wiring.settle({
      state: "unreachable",
      fleet,
      detail: "the socket has not answered",
      sinceMs: this.unreachableSince,
    });
  }

  private unwatch(): void {
    if (this.watch !== null) clearTimeout(this.watch);
    this.watch = null;
  }

  /** A drop says so. It never leaves stale state reading as live. */
  private dropped(fleet: BridgeStateFleet, detail: string): void {
    if (this.socket === null || this.stopped) return;
    this.unwatch();
    this.socket.removeAllListeners();
    this.socket = null;
    if (this.unreachableSince === null) this.unreachableSince = this.wiring.now();
    this.wiring.settle({ state: "unreachable", fleet, detail, sinceMs: this.unreachableSince });
    this.later();
  }

  private later(): void {
    if (this.stopped || this.retry !== null) return;
    this.retry = setTimeout(() => {
      this.retry = null;
      void this.attach();
    }, RETRY_MS);
  }
}
