// Where Fleet is, and why it is not answering.
//
// **Not about Electron.** A runtime file at a path, a pid, a port and a
// protocol ID are what any client of Fleet reads to find it, so they sit
// with the wire rather than with the process that happens to read them here.
// How the answer reaches a renderer is `apps/desktop`'s business and is not
// described here.

/** Injected at build from the wire surface (`apps/desktop/codegen/protocol-id.mjs`); absent under a test runner. */
declare const __PROTOCOL_ID__: string | undefined;

/** The protocol this Bridge was built with. `unbuilt` where nothing injected one, which no Fleet speaks. */
export const PROTOCOL_ID: string = typeof __PROTOCOL_ID__ === "string" ? __PROTOCOL_ID__ : "unbuilt";

/**
 * Which wire a build speaks, as `crates/ipc` hashes it. Compared whole: two
 * IDs are equal or the sides do not talk, and there is no order between them.
 * `""` is a runtime file written before IDs, which equals no real one.
 */
export type ProtocolId = string;

/** Whether Fleet speaks the protocol this Bridge was built with. */
export function speaksOurProtocol(fleet: ProtocolId): boolean {
  return fleet === PROTOCOL_ID;
}

/** How an ID is written on screen: its first eight digits, or `unknown` where there is none. */
export function spoken(id: ProtocolId): string {
  return id === "" ? "unknown" : id.slice(0, 8);
}

/**
 * What pressing Restart Fleet came to. Bridge asks launchd to restart the job
 * that runs Fleet, so the answer is launchd's and not Fleet's: a Fleet that
 * predates the button answers it as well as any other.
 *
 * `ok` means launchd took the request, not that Fleet is back or that it now
 * matches this Bridge. The next runtime file says which.
 */
export type FleetRestart =
  | { ok: true }
  | {
      ok: false;
      /**
       * `not_started_by_armada`: no launchd job of ours holds the pid in the
       * runtime file. `not_running`: the file names no live Fleet. `refused`:
       * launchd answered an error. `no_answer`: it took the request and no new
       * Fleet appeared in time (set by the window, never by main).
       */
      why: "not_started_by_armada" | "not_running" | "refused" | "no_answer";
      detail: string;
    };

/** Fleet, as its runtime file names it. Loopback plus `port` is the address. */
export type FleetIdentity = {
  protocolId: ProtocolId;
  pid: number;
  port: number;
  /** `ps -o lstart=` as it read when Fleet published the file. */
  startedAt: string;
};

/**
 * Why the runtime file does not describe a live Fleet.
 *
 * Three, not one. Bridge renders the first two as "Fleet is not running" and
 * says which under it, because the third — a pid something else now holds — is
 * the case a bare liveness check gets wrong, and the consequence is a socket
 * opened against a port an unrelated program owns.
 */
export type Absence =
  | { why: "no_runtime_file"; path: string }
  | { why: "pid_dead"; path: string; pid: number }
  | { why: "pid_held_by_another"; path: string; pid: number; wrote: string; holder: string };

/**
 * Why the runtime file could not be read at all. **None of these is "not
 * running"** — that is a fact about the world, and folding a failed read into
 * it tells a person Fleet is down on no evidence.
 */
export type RuntimeFault =
  | { why: "unreadable"; path: string; detail: string }
  | { why: "undecodable"; path: string; detail: string }
  | { why: "probe_failed"; path: string; pid: number; detail: string };

/** Where Bridge's one connection is. */
export type Connection =
  | { state: "reading" }
  | { state: "not_running"; absence: Absence }
  | { state: "runtime_file_refused"; fault: RuntimeFault }
  /** The first attempt on a Fleet whose pid checks out. Milliseconds on a Fleet that has been up. */
  | { state: "connecting"; fleet: FleetIdentity }
  /**
   * The pid checks out, the process is young, and the socket has not answered.
   * Fleet binds its port, publishes the runtime file and only then reconciles,
   * so for that whole stretch a connection is accepted by the kernel and
   * never answered. Once the process is older than Bridge allows a boot, this
   * becomes `unreachable`.
   */
  | { state: "starting"; fleet: FleetIdentity }
  /** The pid checks out and the socket does not answer. A different thing to do. */
  | { state: "unreachable"; fleet: FleetIdentity; detail: string; sinceMs: number }
  /**
   * Fleet speaks another protocol than this Bridge, so Bridge opens no socket.
   * Either side may be the stale one and the IDs do not say which, except that
   * a `speaks` of `""` is a Fleet from before IDs.
   */
  | { state: "protocol_mismatch"; fleet: FleetIdentity; speaks: ProtocolId; expected: ProtocolId }
  /** Connected, and both sides speak the same protocol. */
  | { state: "connected"; fleet: FleetIdentity; cursor: number };

/** A live connection. Here so every surface builds `connected` one way. */
export function connectedTo(fleet: FleetIdentity, cursor: number): Connection {
  return { state: "connected", fleet, cursor };
}

/* What the side holding the connection knows about its own session, which the
 * side rendering a failure cannot derive.
 *
 * **Here rather than with the app that fills it.** Both a host process and the
 * surface that draws its failures need this type, and they sit at opposite ends
 * of the layering — the only place both can reach is the bottom. It is the
 * loosest thing in this package: an audit path is a local file, not something
 * Fleet ever sends. It travels because a shared vocabulary is what this package
 * is for, not because a socket carries it. */
/**
 * What every Bridge failure carries that is not about the failure — where the
 * machine log is, and which Fleet is on the other end of the one connection.
 *
 * **No `run_id`, and none is minted.** The envelope makes `run_id` the one id an
 * emitter mints for itself, but nothing in Bridge writes a log line yet, so an
 * id minted here would join to nothing and would read on screen as though it
 * identified the failure. The only real one is the one a `WireError` carries,
 * and that names Fleet's run rather than any single failure.
 *
 * Both fields are facts main holds and the renderer cannot derive — a home
 * directory it cannot resolve, and a connection it does not own — so both are
 * published rather than guessed at.
 */
export type BridgeIdentity = {
  /** The machine log. `null` where HOME is not set and no path resolves. */
  auditPath: string | null;
  /**
   * The protocol Fleet speaks, as `spoken` writes it, as the runtime file said it.
   *
   * **Here rather than at each failure, because four of the five failures are
   * handed no connection.** A refusal is the case that made it worth fixing:
   * Fleet answered it, so Fleet's ID is the first thing a reader of the
   * payload wants, and it was the one payload guaranteed to omit it. Derived
   * where the connection is published, so nothing re-derives it per failure.
   *
   * `null` before a runtime file has been read and believed, and again the
   * moment the connection is one of the states that never got a version —
   * which is a fact rather than a gap, and the tail omits the row.
   */
  fleetProtocol: string | null;
};
