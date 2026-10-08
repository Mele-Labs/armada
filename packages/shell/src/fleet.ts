// What Fleet is, in the words the design contract settles. The Fleet panel
// draws them; this decides them.
//
// **A healthy status is stated out loud**, because an empty panel reads the
// same whether Fleet is healthy or dead.
//
// Four runtime-file answers, not one message. "Fleet is not running" is three
// different facts underneath — no file, a dead pid, and a pid something else
// now holds — and the third is the one that must never become a socket.
//
// This was `FleetBar.tsx`, then `Compositions/Status bar`. Bridge/1088 moved
// the drawing to `Compositions/FleetPanel`; what is left here is the reading.

import type { Connection } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import type { FleetState } from "@armada/components";

/** The one thing to do about a mismatch, whichever side is stale. */
const MATCH_THEM = "Run /update-armada, then reopen Bridge.";

/** Sentence, detail and hue. The detail is machine-derived and renders in mono. */
export type Statement = {
  headline: string;
  detail: string;
  /** What to do about it, where there is something to do. */
  next: string | null;
};

// **No hue here any more.** This carried a status token stem per state, and
// the bar drew a dot in it. The contract's bar names three states and the two
// readings that are neither — a refused runtime file, a protocol Bridge does
// not speak — took a fourth hue that the contract does not grant. They keep
// the neutral dot; the sentence names them, and the failure notice on the
// board carries the whole reading.

export function elapsed(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  return minutes < 60 ? `${minutes}m` : `${Math.round(minutes / 60)}h`;
}

export function statementOf(connection: Connection, now: number, readAt: number | null): Statement {
  switch (connection.state) {
    case "reading":
      return {
        headline: "Reading Fleet's runtime file",
        detail: "",
        next: null,
      };

    case "not_running":
      return {
        headline: "Fleet is not running",
        detail: absence(connection.absence),
        // Bridge cannot start Fleet, so the sentence says who does.
        next: "Start Fleet. Bridge reconnects on its own.",
      };

    case "runtime_file_refused":
      return {
        headline: "Fleet's runtime file was refused",
        detail: `${connection.fault.why}: ${connection.fault.detail} · ${connection.fault.path}`,
        // Not folded into "not running": the read failed, and calling that a
        // Fleet that is down decides on no evidence.
        next: "Check what wrote the file. Bridge will not connect to a port it names.",
      };

    case "connecting":
      return {
        headline: "Connecting to Fleet",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port}`,
        next: null,
      };

    case "starting":
      return {
        headline: "Fleet is starting",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port}`,
        next: null,
      };

    case "unreachable":
      return {
        headline: "Fleet unreachable",
        detail: `pid ${connection.fleet.pid} alive on port ${connection.fleet.port}, ${silenceOf(connection, now, readAt)}`,
        // Restarting Fleet is the wrong fix here, so the sentence does not say to.
        next: "Fleet is up and not answering. What is shown below is not live.",
      };

    case "protocol_mismatch": {
      const sides = `Fleet ${spoken(connection.speaks)} · Bridge ${spoken(connection.expected)}`;
      // The IDs carry no order, so which side is stale is known only for a Fleet
      // whose runtime file has no ID: that one is from before them.
      return connection.speaks === ""
        ? { headline: "Fleet is out of date", detail: sides, next: MATCH_THEM }
        : { headline: "Fleet and Bridge do not match", detail: sides, next: MATCH_THEM };
    }

    case "connected":
      // **No "last read" here, and that is the point.** A healthy connection
      // folds `job.created`, `job.state_changed` and `job.step_advanced` as
      // they arrive, so the Board is current by construction and an age beside
      // it says the opposite of what is true. The age stays on `unreachable`,
      // which is the state where how old the reading is is the whole fact.
      return {
        headline: "Fleet running",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port}`,
        next: null,
      };
  }
}

/**
 * How long an unreachable Fleet has been silent, and how old what is shown is —
 * `no answer for 20s · last read 4s ago`. The Fleet panel draws pid and port as
 * rows of their own and this as the sentence under them, so the panel and the
 * failure notice say it in one spelling.
 */
export function silenceOf(
  connection: Extract<Connection, { state: "unreachable" }>,
  now: number,
  readAt: number | null,
): string {
  const staleness = readAt === null ? "nothing read yet" : `last read ${elapsed(now - readAt)} ago`;
  return `no answer for ${elapsed(now - connection.sinceMs)} · ${staleness}`;
}

/** The three ways a runtime file says Fleet is not running. */
function absence(why: NotRunning): string {
  switch (why.why) {
    case "no_runtime_file":
      return `no runtime file at ${why.path}`;
    case "pid_dead":
      return `pid ${why.pid} is held by nothing · Fleet exited without cleaning up`;
    case "pid_held_by_another":
      // The row a bare liveness check gets wrong.
      return (
        `pid ${why.pid} is held by a process that started ${why.holder}, ` +
        `not ${why.wrote} · its port is not Fleet's`
      );
  }
}

type NotRunning = Extract<Connection, { state: "not_running" }>["absence"];

/**
 * Which of the Fleet panel's dot hues this reading takes.
 *
 * Four of Bridge's eight connection states are none of the contract's three —
 * reading, connecting, a refused runtime file and a protocol Bridge does not
 * speak. Those keep the neutral dot and `shortLabelOf` names each one instead;
 * `starting` is a Fleet booting, which draws a glyph in place of the dot.
 */
export function fleetStateOf(connection: Connection): FleetState {
  switch (connection.state) {
    case "connected":
      return "running";
    case "not_running":
      return "not-running";
    case "starting":
      return "starting";
    case "unreachable":
      return "unreachable";
    default:
      return "unknown";
  }
}

/**
 * The Fleet panel's own word for each connection state. **Exported since
 * #1437**: the title row's folded dot speaks this word too, in its accessible
 * name and its tooltip, and a test that retyped "Running" would pass on the
 * day the panel started saying something else.
 */
export const SHORT_LABEL: Record<Connection["state"], string> = {
  reading: "Reading",
  not_running: "Not running",
  runtime_file_refused: "Refused",
  connecting: "Connecting",
  starting: "Starting",
  unreachable: "Unreachable",
  protocol_mismatch: "Mismatch",
  connected: "Running",
};

/** The Fleet panel's own big word — short, because the panel already says "Fleet". */
export function shortLabelOf(connection: Connection): string {
  return SHORT_LABEL[connection.state];
}
