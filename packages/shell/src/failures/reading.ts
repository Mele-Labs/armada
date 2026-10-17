// What Bridge could not read: Fleet.
//
// **Not provoked by a press.** It is read off published state on every render, which is what
// separates it from the answer to a command.
//
// **Both error classes appear here and nowhere else.** The line falls exactly where the pid check
// falls; each declaration argues its own.

import type {
  BridgeCode,
  DebugField,
  FailureDetail,
} from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import type { Connection } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import { elapsed } from "../fleet";
import type { Statement } from "../fleet";
import type { Failure, FailureFacts } from "./notice";
import { logField, machineLog, versions } from "./notice";

/**
 * Fleet's runtime file named no process Bridge could connect to. **A fault**:
 * no Fleet is running, so no Job is progressing and nothing becomes current on
 * its own. The fix is the one a degraded notice must never send anybody to.
 */
const FLEET_NOT_RUNNING: BridgeCode = "bridge.fleet.not_running";

/**
 * Fleet's runtime file could not be read, so its liveness is unknown.
 *
 * **A fault, and the one the rule has to work on.** It sits beside two degraded
 * states and looks like them. Degraded asserts Fleet is alive, and here the
 * read that would have established it failed — an unknown is not a claim, and
 * drawing it as a stale view says wait for a daemon that may not exist.
 */
const FLEET_RUNTIME_FILE_REFUSED: BridgeCode = "bridge.fleet.runtime_file_refused";

/**
 * Fleet's process is alive and its socket has stopped answering.
 *
 * **Degraded**, and the case the design system names by hand. The pid was
 * verified, so Fleet is running and Jobs keep progressing; what stopped is
 * Bridge's reading of them. This builder's own note has said so all along —
 * "Jobs keep progressing either way" — while the notice above it drew red.
 */
const FLEET_UNREACHABLE: BridgeCode = "bridge.fleet.unreachable";

/**
 * Fleet speaks a protocol Bridge will not open a socket for.
 *
 * **Degraded.** The pid was verified and the socket answered with a version, so
 * Fleet is alive and dispatching; Bridge declined to read it rather than failed
 * to. Independent lifetimes are the point, so a person has a stale board rather
 * than a stopped fleet.
 *
 * **Standing rather than self-clearing, which does not make it a fault.** The
 * class is about whether the work is still happening, not about recovery.
 */
const FLEET_PROTOCOL_MISMATCH: BridgeCode = "bridge.fleet.protocol_mismatch";

/**
 * Fleet, when the one connection is not a connection.
 *
 * `null` where there is nothing wrong. A state with no `next` has nothing to
 * say and takes no notice; a state that is not a fault takes none either, which
 * is why `connected` is answered below rather than filtered out by the guard.
 *
 * **The four runtime-file answers stay four.** Which one it was is the first
 * row of the fold, because only one of the four — running and silent — is
 * worth waiting on, and the other three need somebody to start Fleet.
 *
 * **This is the builder where both classes appear**, and the line falls exactly
 * where the pid check falls: `unreachable` and `protocol_mismatch` are the two where
 * Bridge verified the pid, so Fleet is known alive and only the reading has
 * stopped. The other two draw red because Fleet is either absent or unproven.
 */
export function fleetFailure(
  connection: Connection,
  statement: Statement,
  bridge: BridgeIdentity,
  now: number,
): Failure | null {
  if (statement.next === null) return null;

  const base = {
    headline: statement.headline,
    next: statement.next,
    // Bridge never reached Fleet, so there is no Fleet run to quote. What
    // identifies these is in the fold: the runtime file, the pid and the port.
    values: machineLog(bridge),
  };

  /**
   * The connection state is the machine's own name for what happened, and it
   * leads the fields for that reason: `not_running` and `unreachable` are two
   * different things to do about it, and the sentence above renders both as
   * Fleet being unavailable.
   *
   * **A minted code and no run id**, which is not an inconsistency: a code
   * names a kind of failure and Bridge knows which of the four this is, while
   * a run id names a process instance and the one that would be quoted here is
   * Fleet's, which Bridge never reached. The four codes are four because the
   * four answers are four — `not_running` and `unreachable` are opposite fixes,
   * and one code over both would be the generic message this file exists to
   * repair, moved into the field a person quotes.
   */
  function facts(code: BridgeCode, fields: DebugField[]): FailureFacts {
    return {
      code,
      message: statement.headline,
      fields: [
        { key: "connection", value: connection.state },
        ...fields,
        ...logField(bridge),
      ],
      ...versions(bridge),
    };
  }

  switch (connection.state) {
    case "not_running": {
      const absence = connection.absence;
      const details: FailureDetail[] = [
        { label: "Answer", value: absence.why },
        { label: "Runtime file", value: absence.path },
      ];
      if (absence.why === "pid_dead" || absence.why === "pid_held_by_another") {
        details.push({ label: "Pid", value: String(absence.pid) });
      }
      if (absence.why === "pid_held_by_another") {
        details.push({ label: "File wrote", value: absence.wrote });
        details.push({ label: "Holder started", value: absence.holder });
      }
      const absent: DebugField[] = [
        { key: "why", value: absence.why },
        { key: "runtime_file", value: absence.path },
      ];
      if (absence.why === "pid_dead" || absence.why === "pid_held_by_another") {
        absent.push({ key: "pid", value: String(absence.pid) });
      }
      if (absence.why === "pid_held_by_another") {
        absent.push({ key: "file_wrote", value: absence.wrote });
        absent.push({ key: "holder_started", value: absence.holder });
      }
      return {
        ...base,
        kind: "fault",
        payload: facts(FLEET_NOT_RUNNING, absent),
        detailsLabel: "What the runtime file answered",
        details,
        note:
          absence.why === "pid_held_by_another"
            ? "Bridge did not open a socket. The port in this file is not Fleet's."
            : "Bridge rereads the file every 2 seconds and connects when Fleet writes one.",
      };
    }

    case "runtime_file_refused":
      return {
        ...base,
        kind: "fault",
        payload: facts(FLEET_RUNTIME_FILE_REFUSED, [
          { key: "why", value: connection.fault.why },
          { key: "runtime_file", value: connection.fault.path },
          { key: "detail", value: connection.fault.detail },
          ...(connection.fault.why === "probe_failed"
            ? [{ key: "pid", value: String(connection.fault.pid) }]
            : []),
        ]),
        detailsLabel: "What the read answered",
        details: [
          { label: "Answer", value: connection.fault.why },
          { label: "Runtime file", value: connection.fault.path },
          { label: "Detail", value: connection.fault.detail },
          ...(connection.fault.why === "probe_failed"
            ? [{ label: "Pid", value: String(connection.fault.pid) }]
            : []),
        ],
        // Not folded into "not running": the read failed, and calling that a
        // Fleet that is down decides on no evidence.
        note: "The read failed, so whether Fleet is running is unknown. Bridge will not connect to a port this file names.",
      };

    case "unreachable":
      return {
        ...base,
        kind: "degraded",
        payload: facts(FLEET_UNREACHABLE, [
          { key: "pid", value: String(connection.fleet.pid) },
          { key: "port", value: String(connection.fleet.port) },
          { key: "silent_for", value: elapsed(now - connection.sinceMs) },
          { key: "detail", value: connection.detail },
        ]),
        detailsLabel: "What the connection answered",
        details: [
          { label: "Pid", value: String(connection.fleet.pid) },
          { label: "Port", value: String(connection.fleet.port) },
          { label: "Silent for", value: elapsed(now - connection.sinceMs) },
          { label: "Detail", value: connection.detail },
        ],
        note: "Bridge is retrying every 2 seconds. Jobs keep progressing either way.",
      };

    case "protocol_mismatch":
      return {
        ...base,
        kind: "degraded",
        payload: facts(FLEET_PROTOCOL_MISMATCH, [
          // Not the tail's `fleet protocol`, and not always equal to it: the
          // tail carries what the runtime file said, and this is what the
          // socket said. A Fleet restarted under a live connection is not the
          // one the file described, and the two disagreeing is the whole
          // finding.
          { key: "fleet_speaks", value: spoken(connection.speaks) },
          { key: "pid", value: String(connection.fleet.pid) },
          { key: "port", value: String(connection.fleet.port) },
        ]),
        detailsLabel: "What each side speaks",
        details: [
          { label: "Fleet", value: spoken(connection.speaks) },
          { label: "Bridge", value: spoken(connection.expected) },
          { label: "Pid", value: String(connection.fleet.pid) },
          { label: "Port", value: String(connection.fleet.port) },
        ],
        note: "Bridge did not connect. Jobs keep running.",
      };

    // None of the three is a fault, so none takes a notice. Listed rather than defaulted, so a new connection state is a compile
    // error instead of a silent fall-through to a generic message.
    case "reading":
    case "connecting":
    case "starting":
    case "connected":
      return null;
  }
}
