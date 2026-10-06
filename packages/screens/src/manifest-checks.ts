// The Checks page's reading: every Check the selected Manifest has had requested or run, off the
// checkout's run sheet and its run list, which are what the Manifest surface already holds.
//
// **A Check is a run whose name the sheet declares under `checks`.** Commands and Setup share the
// run list and are not Checks, and a workspace's own entries are its own file's. **Requested
// is Verify's waiting steps**: nothing else on the wire says a Check was asked for and has not
// started. No new read, no new wire shape.

import { requesterOf, type CheckoutRunRecord, type CheckoutRunSheet, type LandCheckAt, type ManifestCheckLog, type ManifestCheckRow, type MergeLine, type Requester } from "@armada/protocol";
import { CHECK_OUTCOME } from "@armada/components";
import type { CheckDetail, CheckListRow, CheckListStatus } from "@armada/components";

import { absoluteOf, clockOf, instant, lasting } from "./duration";
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
  /** When a reported Check ended. A checkout run's end is on its record. */
  finishedAt?: string;
  /** Which run of its step, on a reported Check. */
  attempt?: number;
  /** How long an asked run took. A gate row has none. */
  tookMs?: number;
  /** The Job a reported Check belongs to, which its logs are read under. */
  job?: { id: string; handle: string };
  /** Each of a reported Check's logs, `get_check_output`'s `kept`. */
  logs?: readonly ManifestCheckLog[];
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
 * Every Check the sheet declares that has been asked for or run, as one list ordered by when each
 * was requested, newest first (`requestedOf`). **Nothing here for a Check nobody asked for**, which is the rows' whole
 * point against the Manifest surface's declared list.
 */
export function checkEntriesOf(
  sheet: CheckoutRunSheet | undefined,
  runs: readonly CheckoutRunRecord[],
  lines: readonly MergeLine[] = [],
  reported: readonly ManifestCheckRow[] = [],
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
      out.set(step.run_id, { id: step.run_id, name: step.name, status: "running", command: step.run, runId: step.run_id, requestedAt: verify!.started_at, requester: requesterOf(verify!) });
    } else if (step.state === "ran" && !finished.has(step.record.id)) {
      finished.set(step.record.id, step.record);
    }
  }
  const ended = [...finished.values()].filter((record) => !out.has(record.id)).map(entryOfRecord);
  return byRequested([...out.values(), ...waiting, ...land, ...ended, ...reported.map(entryOfReported)]);
}

/**
 * When a Check was asked for: its own start, else when Verify asked for it, else when its ruling
 * was written (a gate row has only that). A merge line Check not started takes its turn's latest
 * start; with none at all it reads as the newest, since the line is running it now.
 */
export function requestedOf(entry: CheckEntry): number {
  const at = entry.startedAt ?? entry.requestedAt ?? entry.finishedAt;
  return at === undefined ? Number.POSITIVE_INFINITY : (instant(at) ?? Number.NEGATIVE_INFINITY);
}

/** Newest request first; equal times keep the order they were gathered in. */
function byRequested(entries: CheckEntry[]): CheckEntry[] {
  return entries
    .map((entry, at) => ({ entry, at, when: requestedOf(entry) }))
    .sort((one, two) => (two.when === one.when ? one.at - two.at : two.when > one.when ? 1 : -1))
    .map((one) => one.entry);
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
      (row.checks ?? []).map((check, _at, all): CheckEntry => {
        // A Check not started has no time of its own: it was asked with the turn, so it takes the
        // latest start among its branch's other Checks. With none, `requestedOf` puts it on top.
        const turn = all.flatMap((one) => (one.started_at === undefined ? [] : [one.started_at])).sort().at(-1);
        const held = LAND_STATE[check.state] ?? { status: "waiting" as const };
        return {
          id: `land:${one.root}:${row.branch}:${check.name}`,
          name: check.name,
          status: held.status,
          command: "",
          requester: requesterOf(check),
          land: { root: one.root, branch: row.branch, check: check.name },
          ...(check.started_at === undefined ? (turn === undefined ? {} : { requestedAt: turn }) : { startedAt: check.started_at }),
          ...(held.says === undefined ? {} : { says: held.says }),
        };
      }),
    ),
  );
}

/** Where a press on who asked goes: a Job, at a step or a Drone, or the merge line at a branch. */
export type AskerOpens = { to: "job"; jobId: string; step?: string; drone?: string; task?: string } | { to: "merge-line"; branch?: string };

/**
 * Who asked, as one line of bare facts, and where pressing it goes. **Every kind but `outside` opens
 * its requester**: a gate its Job at the step, a Drone's task its Job at that task, a Drone's step its Job at that Drone, the merge line at the
 * branch. Absent `opens` is plain text, which is `outside` and any kind with no id to go to.
 */
export type Asker = { label: string; opens?: AskerOpens };

export function askerOf(requester: Requester, jobLabel: (jobId: string) => string): Asker {
  const { kind, job_id: jobId, step, task_id: task, drone_id: drone, branch } = requester;
  const said = (...parts: (string | undefined)[]) => parts.filter((one): one is string => one !== undefined).join(" · ");
  const job = jobId === undefined ? undefined : (requester.handle ?? jobLabel(jobId));
  switch (kind) {
    case "gate":
      return {
        label: said("Gate", job, step),
        ...(jobId === undefined ? {} : { opens: { to: "job", jobId, ...(step === undefined ? {} : { step }) } as const }),
      };
    case "drone_task":
      return {
        label: said(drone === undefined ? "Drone" : `Drone ${drone}`, job, step, task),
        ...(jobId === undefined
          ? {}
          : { opens: { to: "job", jobId, ...(task === undefined ? (drone === undefined ? {} : { drone }) : { task }), ...(task !== undefined || step === undefined ? {} : { step }) } as const }),
      };
    case "drone_step":
      return {
        label: said(drone === undefined ? "Drone" : `Drone ${drone}`, job, step, task),
        ...(jobId === undefined
          ? {}
          : { opens: { to: "job", jobId, ...(drone === undefined ? {} : { drone }), ...(step === undefined ? {} : { step }) } as const }),
      };
    case "merge_line":
      return { label: said("Merge line", branch), opens: { to: "merge-line", ...(branch === undefined ? {} : { branch }) } };
    case "outside":
      return { label: "Started outside a Job" };
    default:
      return { label: said(kind, job, step, task, drone, branch), ...(jobId === undefined ? {} : { opens: { to: "job", jobId } as const }) };
  }
}

/** An asked run's own states, which the gate's outcomes do not share. */
const ASKED_STATE: Record<string, { status: CheckListStatus; says?: string }> = {
  running: { status: "running" },
  passed: { status: "passed" },
  failed: { status: "failed" },
  stopped: { status: "stopped" },
  lost: { status: "stopped", says: "lost" },
};

/** A gate outcome on the marks a row has, said in the registry's own verb where it is not plain. */
function gateStatus(outcome: string): { status: CheckListStatus; says?: string } {
  if (outcome === "passed") return { status: "passed" };
  if (outcome === "failed") return { status: "failed" };
  const verb = CHECK_OUTCOME[outcome]?.verb ?? outcome;
  // Never started and not run are a Check that was not measured: neither a pass nor a failure.
  return outcome === "never_ran" || outcome === "skipped"
    ? { status: "waiting", says: verb }
    : { status: "failed", says: verb };
}

/** One gate or asked-run row off the manifest-wide read, as a list entry. */
export function entryOfReported(row: ManifestCheckRow): CheckEntry {
  const held = row.source === "asked_run" ? (ASKED_STATE[row.state] ?? { status: "waiting" as const }) : gateStatus(row.state);
  const key = row.asked_run_id !== undefined ? `run${row.asked_run_id}` : `${row.step}.${row.attempt}.${row.group ?? ""}.${row.name}`;
  return {
    id: `reported:${row.job_id}:${key}`,
    name: row.name,
    status: held.status,
    command: "",
    requester: row.requester,
    job: { id: row.job_id, handle: row.job_handle },
    ...(row.logs === undefined || row.logs.length === 0 ? {} : { logs: row.logs }),
    ...(held.says === undefined ? {} : { says: held.says }),
    ...(row.started_at === undefined ? {} : { startedAt: row.started_at }),
    ...(row.ended_at === undefined ? {} : { finishedAt: row.ended_at }),
    ...(row.took_ms === undefined ? {} : { tookMs: row.took_ms }),
    attempt: row.attempt,
  };
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
    by: askerOf(entry.requester, jobLabel).label,
    ...(started === undefined ? {} : { started }),
    ...(startedExact === undefined ? {} : { startedExact }),
    ...(record !== undefined
      ? { duration: lasting(record.duration_ms) }
      : entry.tookMs === undefined
        ? {}
        : { duration: lasting(entry.tookMs) }),
  };
}

/** Every fact the Check has, in the order a person reads them. A fact it lacks is not drawn. */
export function checkDetailsOf(entry: CheckEntry, requestedBy?: CheckDetail["value"]): CheckDetail[] {
  const { record } = entry;
  const details: CheckDetail[] = [];
  if (entry.command !== "") details.push({ label: "Command", value: entry.command, mono: true });
  if (requestedBy !== undefined) details.push({ label: "Requested by", value: requestedBy });
  if (entry.land !== undefined) details.push({ label: "Branch", value: entry.land.branch, mono: true });
  if (entry.attempt !== undefined) details.push({ label: "Run of the step", value: String(entry.attempt) });
  if (entry.requestedAt !== undefined) details.push({ label: "Requested", value: absoluteOf(entry.requestedAt) ?? entry.requestedAt });
  if (entry.startedAt !== undefined) details.push({ label: "Started", value: absoluteOf(entry.startedAt) ?? entry.startedAt });
  if (record === undefined && entry.finishedAt !== undefined) details.push({ label: "Ended", value: absoluteOf(entry.finishedAt) ?? entry.finishedAt });
  if (record === undefined && entry.tookMs !== undefined) details.push({ label: "Took", value: lasting(entry.tookMs) });
  if (record === undefined && entry.job !== undefined) details.push({ label: "Job", value: entry.job.handle, mono: true });
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
