// The Checks page's reading: every Check the selected Manifest has had requested or run, off the
// checkout's run sheet and its run list, which are what the Manifest surface already holds.
//
// **A Check is a run whose name the sheet declares under `checks`.** Commands and Setup share the
// run list and are not Checks, and a workspace's own entries are its own file's. **Requested
// is Verify's waiting steps**: nothing else on the wire says a Check was asked for and has not
// started. No new read, no new wire shape.

import { requesterOf, type CheckoutRunRecord, type CheckoutRunSheet, type LandCheckAt, type MergeLine, type Requester } from "@armada/protocol";
import type { CheckDetail, CheckListRow, CheckListStatus } from "@armada/components";

import { absoluteOf, clockOf, lasting } from "./duration";
import { runOutcomeOf } from "./rehearsal";

/** One Check, asked for, out, or ended. */
export type CheckEntry = {
  /** A run's own id; a waiting step has none, so it takes Verify's and its name. */
  id: string;
  name: string;
  status: CheckListStatus;
  command: string;
  /** The run, once there is one. Absent while waiting. */
  runId?: string;
  startedAt?: string;
  /** The finished run's record. */
  record?: CheckoutRunRecord;
  /** When Verify asked for it, on a step still waiting. */
  requestedAt?: string;
  /** Who asked for it. A checkout run is `outside`; a merge line Check is the line's. */
  requester: Requester;
  /** The merge line Check's own names, which its log is asked for by. */
  land?: LandCheckAt;
  /** Said on hover in place of the status's own word, where the wire has a finer one. */
  says?: string;
};

/** What each status says on hover, as the Check list's mark names it. */
const SAYS: Record<CheckListStatus, string> = {
  waiting: "waiting",
  running: "running",
  passed: "passed",
  failed: "failed",
  stopped: "stopped",
};

/**
 * Every Check the sheet declares that has been asked for or run: out now, then waiting, then the
 * finished, newest first. **Nothing here for a Check nobody asked for**, which is the rows' whole
 * point against the Manifest surface's declared list.
 */
export function checkEntriesOf(
  sheet: CheckoutRunSheet | undefined,
  runs: readonly CheckoutRunRecord[],
  lines: readonly MergeLine[] = [],
): CheckEntry[] {
  const land = landEntriesOf(lines);
  if (sheet === undefined) return land;
  const declared = new Set(sheet.checks.map((one) => one.name));
  const isCheck = (run: { name: string; workspace?: string }) => run.workspace === undefined && declared.has(run.name);

  const out = new Map<string, CheckEntry>();
  const running = sheet.running;
  if (running !== undefined && isCheck(running)) {
    out.set(running.id, { id: running.id, name: running.name, status: "running", command: running.command, runId: running.id, startedAt: running.started_at, requester: requesterOf(running) });
  }
  const waiting: CheckEntry[] = [];
  const verify = sheet.verify;
  const finished = new Map<string, CheckoutRunRecord>();
  for (const record of runs) if (isCheck(record)) finished.set(record.id, record);
  for (const step of verify?.steps ?? []) {
    if (step.group !== "checks") continue;
    if (step.state === "waiting") {
      waiting.push({ id: `${verify!.id}:${step.name}`, name: step.name, status: "waiting", command: step.run, requestedAt: verify!.started_at, requester: requesterOf(verify!) });
    } else if (step.state === "running" && !out.has(step.run_id)) {
      out.set(step.run_id, { id: step.run_id, name: step.name, status: "running", command: step.run, runId: step.run_id, requester: requesterOf(verify!) });
    } else if (step.state === "ran" && !finished.has(step.record.id)) {
      finished.set(step.record.id, step.record);
    }
  }
  const ended = [...finished.values()]
    .filter((record) => !out.has(record.id))
    .sort((one, two) => two.started_at.localeCompare(one.started_at))
    .map(entryOfRecord);
  return [...out.values(), ...waiting, ...land, ...ended];
}

function entryOfRecord(record: CheckoutRunRecord): CheckEntry {
  return {
    id: record.id,
    name: record.name,
    status: runOutcomeOf(record),
    command: record.command,
    runId: record.id,
    startedAt: record.started_at,
    record,
    requester: requesterOf(record),
  };
}

/** What a merge line Check's own state maps onto, and what the mark says where the wire is finer. */
const LAND_STATE: Record<string, { status: CheckListStatus; says?: string }> = {
  waiting: { status: "waiting" },
  running: { status: "running" },
  passed: { status: "passed" },
  failed: { status: "failed" },
  timed_out: { status: "failed", says: "timed out" },
};

/**
 * Every Check the merge line's turns run, for each branch that has them. **Only what the line
 * holds**: `checks` rides on a branch gating, red or stopped, so a landed branch has none here.
 */
function landEntriesOf(lines: readonly MergeLine[]): CheckEntry[] {
  return lines.flatMap((one) =>
    [...one.line, ...one.sent_back, ...one.landed, ...one.off].flatMap((row) =>
      (row.checks ?? []).map((check): CheckEntry => {
        const held = LAND_STATE[check.state] ?? { status: "waiting" as const };
        return {
          id: `land:${one.root}:${row.branch}:${check.name}`,
          name: check.name,
          status: held.status,
          command: "",
          requester: requesterOf(check),
          land: { root: one.root, branch: row.branch, check: check.name },
          ...(held.says === undefined ? {} : { says: held.says }),
        };
      }),
    ),
  );
}

/**
 * Who asked, as the words and the one place a press goes. **A bare fact per kind**, joined by the
 * caller; the Job and the merge line are the two places a route exists, so only they link. A step,
 * a task and a Drone are named beside the link and are not links: nothing opens a Job at one.
 */
export type Asker = { parts: (string | { link: "job" | "merge-line"; label: string; jobId?: string })[] };

export function askerOf(requester: Requester, jobLabel: (jobId: string) => string): Asker {
  const { kind, job_id: jobId, step, task_id: task, drone_id: drone, branch } = requester;
  const job = jobId === undefined ? [] : [{ link: "job" as const, label: jobLabel(jobId), jobId }];
  const named = (...parts: (string | undefined)[]) => parts.filter((one): one is string => one !== undefined);
  switch (kind) {
    case "gate":
      return { parts: ["Gate", ...job, ...named(step)] };
    case "drone_task":
      return { parts: [drone === undefined ? "Drone" : `Drone ${drone}`, ...job, ...named(step, task)] };
    case "drone_step":
      return { parts: [drone === undefined ? "Drone" : `Drone ${drone}`, ...job, ...named(step)] };
    case "merge_line":
      return { parts: [{ link: "merge-line", label: "Merge line" }, ...named(branch)] };
    case "outside":
      return { parts: ["Started outside a Job"] };
    default:
      return { parts: [...named(kind), ...job, ...named(step, task, drone, branch)] };
  }
}

/** One row of the list. */
export function checkRowOf(entry: CheckEntry, jobLabel: (jobId: string) => string = (id) => id): CheckListRow {
  const { record } = entry;
  const started = entry.startedAt === undefined ? undefined : clockOf(entry.startedAt);
  const startedExact = entry.startedAt === undefined ? undefined : (absoluteOf(entry.startedAt) ?? undefined);
  return {
    id: entry.id,
    name: entry.name,
    status: entry.status,
    says: entry.says ?? SAYS[entry.status],
    by: askerOf(entry.requester, jobLabel)
      .parts.map((one) => (typeof one === "string" ? one : one.label))
      .join(" · "),
    ...(started === undefined ? {} : { started }),
    ...(startedExact === undefined ? {} : { startedExact }),
    ...(record === undefined ? {} : { duration: lasting(record.duration_ms) }),
  };
}

/** Every fact the Check has, in the order a person reads them. A fact it lacks is not drawn. */
export function checkDetailsOf(entry: CheckEntry, requestedBy?: CheckDetail["value"]): CheckDetail[] {
  const { record } = entry;
  const details: CheckDetail[] = [];
  if (entry.command !== "") details.push({ label: "Command", value: entry.command, mono: true });
  if (requestedBy !== undefined) details.push({ label: "Requested by", value: requestedBy });
  if (entry.land !== undefined) details.push({ label: "Branch", value: entry.land.branch, mono: true });
  if (entry.requestedAt !== undefined) details.push({ label: "Requested", value: absoluteOf(entry.requestedAt) ?? entry.requestedAt });
  if (entry.startedAt !== undefined) details.push({ label: "Started", value: absoluteOf(entry.startedAt) ?? entry.startedAt });
  if (record !== undefined) {
    details.push({ label: "Ended", value: absoluteOf(record.ended_at) ?? record.ended_at });
    details.push({ label: "Took", value: lasting(record.duration_ms) });
    details.push(
      record.exit_code === undefined
        ? { label: "Ended by", value: record.ended }
        : { label: "Exit", value: `${record.exit_code}, expects ${record.expect_exit_code}`, mono: true },
    );
    if (record.required.length > 0) details.push({ label: "Ran after", value: record.required.join(", "), mono: true });
    if (record.changed.length > 0) {
      details.push({
        label: "Changed",
        value: record.changed.map((file) => file.path).join("\n"),
        mono: true,
      });
    }
  }
  if (entry.runId !== undefined) details.push({ label: "Run", value: entry.runId, mono: true });
  return details;
}
