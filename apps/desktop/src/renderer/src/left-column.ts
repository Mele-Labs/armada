// The left column's Fleet panel, built here rather than in
// `@armada/shell` — that package is what `@armada/screens` imports
// `statementOf` from, so the reverse import would be a dependency cycle.
// Reuses Overview's own tile arithmetic rather than re-deriving it, so the
// two readings cannot drift apart. Bridge/1088.

import type { Figure, FleetPanelProps } from "@armada/components";
import type { Connection } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import { fleetStateOf, shortLabelOf, silenceOf, type Statement } from "@armada/shell";
import { doctorReading } from "@armada/overview";
import type { HealthRead } from "@armada/screens/src/overview-reads";
// `instant` and `lasting` are the Job elapsed-time figure's own parse-and-format
// pair (`elapsedSince` above them). Reused rather than re-derived so a job's
// "1h 30m" and Fleet's "up 1h 30m" cannot drift into two spellings of one span.
import { instant, lasting } from "@armada/screens/src/duration";

/** Every reader `fleetPanelOf` calls returns a plain string `detail`; anything else is dropped rather than stringified blind. */
function asString(node: unknown): string | undefined {
  return typeof node === "string" ? node : undefined;
}

const DOCTOR_OUTCOMES = new Set(["pass", "warn", "fail"]);

/**
 * Fleet — Running, the pid / port / protocol / up rows, and Doctor as a rollup
 * of what `GET /health` answered. **Not Doctor's own grid**, which is unbuilt
 * (#99): the worst of the probes Fleet itself can run, the same reading
 * Overview's own Doctor tile already computes.
 */
export function fleetPanelOf(
  connection: Connection,
  statement: Statement,
  health: HealthRead,
  now: number,
  readAt: number | null,
): Omit<FleetPanelProps, "open" | "onOpenChange"> {
  const reading = doctorReading(health);
  const outcome = asString(reading.value);
  const doctor =
    outcome === undefined || !DOCTOR_OUTCOMES.has(outcome)
      ? undefined
      : { outcome: outcome as "pass" | "warn" | "fail", checked: asString(reading.detail) ?? "" };
  return {
    state: fleetStateOf(connection),
    label: shortLabelOf(connection),
    rows: rowsOf(connection, now),
    detail: sentenceOf(connection, statement, now, readAt),
    doctor,
  };
}

/**
 * pid, port, protocol and up, one row each — **only the ones this state has a
 * value for.** Settled 2026-09-17. A runtime file names pid and port, so a
 * Fleet being connected to or not answering has those two; protocol and uptime
 * are read only once a connection holds, as they were on the old meta line.
 *
 * **Ticks off the app's one `now`** (`App.tsx`'s `setInterval`, already
 * running for every other elapsed figure on screen) rather than a second
 * clock. The uptime is relative to `FleetIdentity.startedAt` — the instant
 * `ps -o lstart=` gave Fleet's own process, the spelling `crates/fleet`
 * chose specifically so Bridge could parse it too — not a static read taken
 * once at connect time. A `startedAt` that will not parse drops the row.
 */
function rowsOf(connection: Connection, now: number): Figure[] | undefined {
  if (
    connection.state !== "connected" &&
    connection.state !== "connecting" &&
    connection.state !== "starting" &&
    connection.state !== "unreachable"
  ) {
    return undefined;
  }
  const rows: Figure[] = [
    { label: "pid", value: String(connection.fleet.pid) },
    { label: "port", value: String(connection.fleet.port) },
  ];
  if (connection.state !== "connected") return rows;
  rows.push({ label: "protocol", value: spoken(connection.fleet.protocolId) });
  const startedMs = instant(connection.fleet.startedAt);
  if (startedMs !== null) rows.push({ label: "up", value: lasting(now - startedMs) });
  return rows;
}

/**
 * The line under the rows, where a state has more to say than its figures.
 * **Never the pid or port again**: the states that have rows say only what the
 * rows cannot, and the rest keep the statement's own detail whole.
 */
function sentenceOf(connection: Connection, statement: Statement, now: number, readAt: number | null) {
  switch (connection.state) {
    case "connecting":
    case "connected":
    case "starting":
      return undefined;
    case "unreachable":
      return `alive, ${silenceOf(connection, now, readAt)}`;
    default:
      return statement.detail === "" ? undefined : statement.detail;
  }
}
