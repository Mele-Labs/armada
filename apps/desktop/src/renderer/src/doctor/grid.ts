// Doctor's grid, read from what Bridge already holds: `GET /health`'s probes, and Bridge's own
// connection for the Armada API row. **Doctor owns no probe logic** (`docs/concepts/doctor.md`), so
// this only orders, names and places what came back. No React, so a node test reads it.

import type { Connection } from "@armada/protocol";
import type { HealthRead } from "@armada/screens/src/overview-reads";
import { said as saidOf } from "@armada/screens/src/copy";

/** Doctor's three words, and the fourth for a row whose probe is in flight. */
export type Result = "pass" | "warn" | "fail" | "reading";

/** One module's row. */
export type Row = {
  /** A stable key: a module with several probes (one Manifest per served repository) draws a row each. */
  key: string;
  module: string;
  result: Result;
  /** What the probe read, in one line. */
  detail: string;
  /** Who answers for it: the crate its probe lives beside, or Bridge for its own connection. */
  owner: string;
};

/** Probes nobody runs yet, as Fleet groups them: by who owns them, with why. Never drawn as a result. */
export type Gap = { owner: string; because: string };

export type Grid = {
  rows: Row[];
  gaps: Gap[];
  /** Why the probes were not read, when they were not. */
  unread?: string;
};

/**
 * The grid in the order `docs/concepts/doctor.md` generates it, with who probes each module —
 * **the modules this side can name.** Those probed from `adapters` are named by their probe when it
 * answers, and until then by Fleet's `not_probed`: their names are vendors', which stay in `adapters`.
 */
export const MODULES: readonly { module: string; owner: string }[] = [
  { module: "Fleet", owner: "fleet" },
  { module: "Armada API", owner: "Bridge" },
  { module: "Kit", owner: "config" },
  { module: "Machine", owner: "config" },
  { module: "Manifest", owner: "fleet" },
  { module: "SQLite", owner: "store" },
  { module: "System stats", owner: "fleet" },
];

/** What each result means for each module — the doc's table, so a picked row can say it. */
export const MEANS: Readonly<Record<string, { reads: string; pass: string; warn?: string; fail: string }>> = {
  Fleet: { reads: "Daemon alive and answering", pass: "Alive, answering", fail: "Not answering" },
  "Armada API": { reads: "Bridge's own connection state, not a probe", pass: "Connected", fail: "Wrong port, stale connection, or a Bridge-side socket failure" },
  Kit: { reads: "kit.yml present, parses, schema current", pass: "Parses, schema current", warn: "Parses, schema behind, migration pending", fail: "Missing or unparseable" },
  Machine: { reads: "machine.yml present, parses, schema current", pass: "Parses, schema current", warn: "Parses, schema behind, migration pending", fail: "Missing or unparseable" },
  Manifest: { reads: "Every known armada.yml parses, schema current, no drift", pass: "All parse, all schemas current", warn: "Some parse and some do not, or some have drifted", fail: "None parse, or none found" },
  SQLite: { reads: "armada.db opens, schema matches, WAL writable", pass: "Opens and writes", warn: "Schema behind, or rows that will not load", fail: "Locked, corrupt, or the volume is full" },
  "System stats": { reads: "CPU, memory and disk headroom against admission's thresholds", pass: "Above threshold", warn: "Below threshold — Drones queue rather than spawn", fail: "Insufficient to run anything" },
};

const WORDS = new Set<Result>(["pass", "warn", "fail"]);

/** Fleet's words, as a result. A word this build does not know is drawn as `warn` rather than dropped. */
function resultOf(outcome: string): Result {
  return WORDS.has(outcome as Result) ? (outcome as Result) : "warn";
}

/** The worst result on the grid, or none while every row passes or reads. */
export function worstOf(rows: readonly Row[]): "fail" | "warn" | undefined {
  if (rows.some((row) => row.result === "fail")) return "fail";
  if (rows.some((row) => row.result === "warn")) return "warn";
  return undefined;
}

/** The Armada API row: Bridge's own socket, never asked of the process it cannot reach. */
function apiRow(connection: Connection, said: string): Row {
  const base = { key: "Armada API", module: "Armada API", owner: "Bridge" };
  switch (connection.state) {
    case "connected":
      return { ...base, result: "pass", detail: `connected on port ${connection.fleet.port}` };
    case "reading":
    case "connecting":
    case "starting":
      return { ...base, result: "reading", detail: said };
    default:
      return { ...base, result: "fail", detail: said };
  }
}

/** Fleet's row when `/health` was not read: what the runtime file and the pid say, from outside. */
function fleetRow(connection: Connection, said: string): Row {
  const base = { key: "Fleet", module: "Fleet", owner: "fleet" };
  switch (connection.state) {
    case "connected":
    case "reading":
    case "connecting":
    case "starting":
      return { ...base, result: "reading", detail: said };
    default:
      return { ...base, result: "fail", detail: said };
  }
}

/**
 * The grid: a row for every module something answered for, in the doc's order, then any module
 * this side does not name (an `adapters` probe, or one newer than this build), and beneath it the
 * probes nobody runs yet.
 *
 * `said` is the connection's own sentence (`statementOf`), so the Fleet and Armada API rows read
 * as the Fleet panel does rather than in a second wording.
 */
export function gridOf(read: HealthRead, connection: Connection, said: string): Grid {
  const health = read.state === "read" ? read.health : undefined;
  const probes = health?.probes ?? [];
  const rows: Row[] = [];
  const gaps: Gap[] = [];
  for (const { module, owner } of MODULES) {
    const answered = probes.filter((probe) => probe.module === module);
    if (module === "Armada API") rows.push(apiRow(connection, said));
    else if (answered.length > 0) {
      answered.forEach((probe, at) =>
        rows.push({ key: at === 0 ? module : `${module}:${at}`, module, result: resultOf(probe.outcome), detail: probe.detail, owner }),
      );
    } else if (module === "Fleet" && health === undefined) rows.push(fleetRow(connection, said));
    // Fleet probes these itself, so they are in flight while `/health` is. A config module is Fleet's `not_probed`.
    else if (owner !== "config" && read.state !== "failed" && health === undefined) rows.push({ key: module, module, result: "reading", detail: "", owner });
  }
  // A module this side does not name still shows: a row dropped is a problem hidden.
  const known = new Set(MODULES.map((one) => one.module));
  probes
    .filter((probe) => !known.has(probe.module))
    .forEach((probe, at) => rows.push({ key: `${probe.module}:x${at}`, module: probe.module, result: resultOf(probe.outcome), detail: probe.detail, owner: "fleet" }));
  // Bridge's own entry is the Armada API row above, drawn rather than excused.
  for (const one of health?.not_probed ?? []) if (one.owner !== "Bridge") gaps.push({ owner: one.owner, because: one.because.replace(/\s+/g, " ") });
  if (read.state === "failed") gaps.unshift({ owner: "fleet", because: `Fleet's own probes were not read: ${unread(read)}` });
  return read.state === "failed" ? { rows, gaps, unread: unread(read) } : { rows, gaps };
}

/** Why `/health` was not read, in the app's own words for that outcome. */
function unread(read: Extract<HealthRead, { state: "failed" }>): string {
  return saidOf(read.outcome) || "Fleet did not answer";
}
