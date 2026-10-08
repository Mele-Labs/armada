// The merge line: the branches waiting to land on main, the ones that just landed, and the ones
// sent back, as Overview's panels and the rail's Merge line surface draw them.
//
// Fleet serves it since protocol 22.1 — `get_merge_lines`, kept current by
// `merge_lines.changed` — reading what `armada land` keeps on disk. `landed` and `sent_back` since
// 23.1. This is the one fold from that wire onto the composition's rows.

import type { HubPull, MainRed, MainState, MergeLineCheck, MergeLineEntry, MergeLineHub, MergeLineNotice, MergeLineState, MergeLineWaiting, RecentJob } from "@armada/components";
import type { HubMerged, JobSummary, MainRun, MainStanding, MergeLine, MergeLineHub as WireHub, MergeLineRow, MergeLines, RepositorySummary } from "@armada/protocol";
import { repositoryLabel } from "@armada/shell";

import { settledBadgeOf } from "./facts";

/** One repository's line, as a panel takes it. */
export type MergeLineView = {
  /** The repository's root: which panel this is. */
  root: string;
  /** The repository, by its label. Only where more than one panel draws. */
  name?: string;
  /** In place order: `--status`'s numbered rows. */
  line: readonly MergeLineEntry[];
  /** Landed, newest first. */
  landed: readonly MergeLineEntry[];
  /** Red, conflict or stopped and not back in line, newest first. */
  sentBack: readonly MergeLineEntry[];
  /** The turn's failed Check, while it runs on. */
  notice?: MergeLineNotice;
  /** Main's state and the repository's open pull requests, where Fleet serves them. */
  hub?: MergeLineHub;
};

/**
 * A line as the mock serves it ahead of the wire. `notice` is not a field of protocol 23 yet, so a
 * Fleet sends none and nothing draws. Folded here so the panel can be walked before it is built.
 */
type NoticedLine = MergeLine & { notice?: MergeLineNotice };

/** A row as the mock serves it ahead of the wire: `why`, the reason a waiting branch is not in the turn. */
type ReasonedRow = MergeLineRow & { why?: MergeLineWaiting };

/**
 * The line in the order it will merge: the turn that is running first, in place order, then what
 * waits, in place order, which is the order the next turn takes. **Each row's number is its
 * position here**, not the place it joined at, which a branch keeps after a red.
 */
function inOrder(rows: readonly MergeLineRow[]): MergeLineRow[] {
  const ahead = (row: MergeLineRow) => (row.state === "waiting" ? 1 : 0);
  return rows
    .map((row, at) => ({ row, at }))
    .sort((a, b) => ahead(a.row) - ahead(b.row) || a.at - b.at)
    .map(({ row }, at) => ({ ...row, place: at + 1 }));
}

/** How much of a merge commit a row shows. */
const SHORT = 10;

/**
 * The panels to draw for this pick, one per repository, in the order Fleet serves them.
 *
 * **A repository draws wherever Fleet serves it a line**, whoever is in it: an empty line still
 * shows what landed and what was sent back. A picked repository draws its own. All draws every
 * one, each named by its repository once there is more than one. None before Fleet answers, or
 * where it serves no line for the pick.
 */
export function mergeLineViews(
  lines: MergeLines | null,
  picked: string | null,
  repositories: readonly RepositorySummary[],
  jobs: readonly JobSummary[] = [],
): readonly MergeLineView[] {
  const served = lines?.lines ?? [];
  const chosen = picked === null ? served : served.filter((one) => one.root === picked);
  const named = chosen.length > 1;
  const owners = ownersOf(jobs);
  const owned = (entry: MergeLineEntry): MergeLineEntry => {
    const job = entry.job ?? owners.get(entry.branch);
    return job === undefined ? entry : { ...entry, job };
  };
  return chosen.map((one) => ({
    root: one.root,
    ...(named ? { name: nameOf(one.root, repositories) } : {}),
    line: inOrder(one.line).map((row) => owned(entryOf(row, (one as NoticedLine).notice?.kind === "main"))),
    // The forge's newest merged pull requests, where Fleet serves them; the queue's own outcome
    // files only from a Fleet before 23.42, or one whose forge said none.
    landed: (one.hub?.merged ?? []).length > 0 ? one.hub!.merged!.map((pull) => owned(mergedEntryOf(pull))) : one.landed.map((row) => owned(entryOf(row))),
    sentBack: one.sent_back.map((row) => owned(entryOf(row))),
    ...((one as NoticedLine).notice === undefined ? {} : { notice: (one as NoticedLine).notice }),
    ...(one.hub === undefined ? {} : { hub: hubOf(one.hub, recentOf(jobs), owners) }),
  }));
}

/** The Job each branch belongs to, from the Board's rows. The newest Job wins where two named one branch. */
function ownersOf(jobs: readonly JobSummary[]): Map<string, { id: string; title: string }> {
  const owners = new Map<string, { id: string; title: string }>();
  for (const job of [...jobs].sort((a, b) => (a.ended_at ?? "\uffff").localeCompare(b.ended_at ?? "\uffff") || a.id.localeCompare(b.id))) {
    if (job.branch !== undefined) owners.set(job.branch, { id: job.id, title: job.title });
  }
  return owners;
}

/** A pull request the forge lists as merged, as a landed row draws it. */
function mergedEntryOf(pull: HubMerged): MergeLineEntry {
  return {
    branch: pull.branch,
    ...(pull.job === undefined ? {} : { job: { id: pull.job.id, title: pull.job.title } }),
    pr: { number: pull.number, url: pull.url },
    state: "landed",
    ...(pull.commit === undefined ? {} : { merge: pull.commit.slice(0, SHORT) }),
    ...(pull.commit === undefined || pull.main_run === undefined ? {} : mainRunOf(pull.main_run, pull.commit)),
  };
}

/** The CI run on the merge commit. A `state` this build does not know draws no mark. */
function mainRunOf(run: MainRun, commit: string): Pick<MergeLineEntry, "mainRun"> {
  if (run.state !== "passed" && run.state !== "running" && run.state !== "failed") return {};
  return { mainRun: { state: run.state, ...(run.failed === undefined || run.failed.length === 0 ? {} : { failed: run.failed }), branch: `main@${commit}` } };
}

/** The statuses work can be sent back to: at its review, or over. */
const SENDABLE = new Set(["awaiting_review", "completed_success", "completed_failed", "killed"]);

/** The Jobs main's red can be sent back to, newest first: the Board's rows that hold a branch. */
function recentOf(jobs: readonly JobSummary[]): readonly RecentJob[] {
  return jobs
    .filter((job) => SENDABLE.has(job.status) && job.branch !== undefined)
    .sort((a, b) => (b.ended_at ?? "").localeCompare(a.ended_at ?? "") || b.id.localeCompare(a.id))
    .map((job) => ({ id: job.id, title: job.title, branch: job.branch! }));
}

/**
 * The hub as the panel draws it. **Main is drawn green or red and nothing else**: a commit still
 * running, or one nothing ran on, has no mark rather than a state the panel has no glyph for. A red
 * with newer commits running is held, and the panel draws it as such.
 */
function hubOf(hub: WireHub, recent: readonly RecentJob[], owners: ReadonlyMap<string, { id: string; title: string }>): MergeLineHub {
  const main = mainOf(hub);
  return {
    ...(main === undefined ? {} : { main }),
    pulls: queueFirst(hub.pull_requests ?? []).map((pull) => ({
      number: pull.number,
      url: pull.url,
      branch: pull.branch,
      ...(pull.ci === undefined ? {} : { ci: pull.ci as NonNullable<HubPull["ci"]> }),
      ...((pull.job ?? owners.get(pull.branch)) === undefined ? {} : { job: pull.job ?? owners.get(pull.branch)! }),
      ...(pull.queue === undefined ? {} : { queue: { state: pull.queue.state as NonNullable<HubPull["queue"]>["state"], ...(pull.queue.position === undefined ? {} : { position: pull.queue.position }) } }),
    })),
    recent,
  };
}

/** The queue first, by position, then the pull requests waiting for ci to join it, then the rest as listed. */
function queueFirst<T extends { queue?: { state: string; position?: number } }>(pulls: readonly T[]): T[] {
  const rank = (pull: T) => (pull.queue?.position !== undefined ? 0 : pull.queue?.state === "waiting_for_ci" ? 1 : 2);
  return pulls
    .map((pull, at) => ({ pull, at }))
    .sort((a, b) => rank(a.pull) - rank(b.pull) || (a.pull.queue?.position ?? 0) - (b.pull.queue?.position ?? 0) || a.at - b.at)
    .map(({ pull }) => pull);
}

function mainOf({ main, fixing }: WireHub): MainState | undefined {
  if (main === undefined) return undefined;
  const checking = (main.checking ?? []).map((one) => ({
    commit: one.commit,
    ...(one.pull_request === undefined ? {} : { number: one.pull_request.number, ...(one.pull_request.url === undefined ? {} : { url: one.pull_request.url }) }),
  }));
  if (main.state === "green") return { state: "green", ...(checking.length === 0 ? {} : { checking }) };
  const red = main.state === "red" ? redOf(main) : undefined;
  if (red === undefined) return undefined;
  return { state: "red", red, ...(fixing === undefined ? {} : { taken: fixing }), ...(checking.length === 0 ? {} : { checking }) };
}

/** What failed: a Manifest Check where the CI job maps to one, otherwise the job's own name. */
function redOf(main: MainStanding): MainRed | undefined {
  const [first, ...rest] = main.failed ?? [];
  if (first === undefined) return undefined;
  const merge = main.merge;
  return {
    check: first.check ?? first.name,
    ...(first.check === undefined ? { unmapped: true as const } : {}),
    ...(first.tests?.[0] === undefined ? {} : { test: first.tests[0] }),
    ...(rest.length === 0
      ? {}
      : {
          also: rest.map((job) => ({
            check: job.check ?? job.name,
            ...(job.check === undefined ? { unmapped: true as const } : {}),
            ...(job.tests?.[0] === undefined ? {} : { test: job.tests[0] }),
          })),
        }),
    ...(merge === undefined
      ? {}
      : {
          merge: {
            number: merge.number,
            ...(merge.url === undefined ? {} : { url: merge.url }),
            ...(merge.branch === undefined ? {} : { branch: merge.branch }),
            ...(merge.job === undefined ? {} : { job: merge.job }),
          },
        }),
  };
}

/** The repository's label, or its root where Fleet's listing has not caught up with its line. */
function nameOf(root: string, repositories: readonly RepositorySummary[]): string {
  const served = repositories.find((one) => one.root === root);
  return served === undefined ? root : repositoryLabel(served, repositories);
}

/**
 * The state a row draws. **A turn that has run no Check yet is `preparing`**, Bridge's own: Fleet
 * serves `gating` for the whole turn, and its Check list is what tells the two parts apart.
 * **A turn whose notice is `main` is `held`**, also Bridge's own: its failed Check is red on main
 * too, so the running rows stop rather than run on.
 */
function stateOf(row: MergeLineRow, held: boolean): MergeLineState {
  if (held && (row.state === "gating" || row.state === "merging")) return "held";
  if (row.state === "gating" && (row.checks ?? []).length === 0) return "preparing";
  return row.state as MergeLineState;
}

/**
 * A row's pull request, with the Job's own badge for how it ended (`settledBadgeOf`), so the merge
 * line and a Job's detail read a settled pull request alike. Nothing known draws the number alone.
 */
function pullRequestOf(pr: NonNullable<MergeLineRow["pull_request"]>): NonNullable<MergeLineEntry["pr"]> {
  const settled = settledBadgeOf(pr.settled);
  return { number: pr.number, url: pr.url, ...(settled === undefined ? {} : { settled }) };
}

function entryOf(row: MergeLineRow, held = false): MergeLineEntry {
  return {
    branch: row.branch,
    place: row.place,
    pr: row.pull_request === undefined ? undefined : pullRequestOf(row.pull_request),
    state: stateOf(row, held),
    why: (row as ReasonedRow).why,
    doing: row.doing,
    batch: row.batch,
    merge: row.merge_commit?.slice(0, SHORT),
    failed: row.failed,
    conflicts: row.conflicts,
    checks: row.checks?.map((check) => ({ name: check.name, state: check.state as MergeLineCheck["state"] })),
  };
}
