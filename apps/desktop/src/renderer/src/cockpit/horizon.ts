// The Cockpit's horizon as one row of chips, nearest main first: Fleet's own landing entries
// (`view.line`), then the pull requests in the forge's merge queue by position, then the rest as the
// line lists them. The pull requests are read off the view's hub (`view.hub.pulls`, which the Merge
// line surface draws) with the title and the owner beside, so the strip folds nothing the surface
// does not. Pure, so the order and the marks are checked without drawing the band.

import { GitMerge, type LucideIcon } from "lucide-react";

import { CHECK_OUTCOME, LAND_STATE, QUEUED_REASON } from "@armada/components";
import type { MergeLines } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";
import type { Session } from "@armada/screens/src/draft/sessions";

import { pullOwner, type Owner } from "./owner";

export type PullState = "queued" | "running" | "failing" | "ready" | "blocked" | "open";

/** The registry's reading for each state; `open` has no mark, since nothing has run on it. */
const MARK = {
  queued: LAND_STATE["waiting"],
  running: LAND_STATE["gating"],
  failing: CHECK_OUTCOME["failed"],
  ready: CHECK_OUTCOME["passed"],
  blocked: QUEUED_REASON["blocked_by_dependency"],
  open: undefined,
} as const;

/** What the merge queue says of a pull request in it, as the Merge line surface words it. */
const QUEUE: Record<string, { state: PullState; says: string }> = {
  waiting_for_ci: { state: "running", says: "Waiting for ci to join the merge queue" },
  in_queue: { state: "queued", says: "In the merge queue" },
  queued: { state: "queued", says: "In the merge queue, waiting its turn" },
  awaiting_checks: { state: "running", says: "In the merge queue, running its checks" },
  mergeable: { state: "ready", says: "In the merge queue, ready to merge" },
  unmergeable: { state: "failing", says: "In the merge queue, cannot merge" },
};

const CI: Record<string, { state: PullState; says: string }> = {
  passed: { state: "ready", says: "ci passed" },
  running: { state: "running", says: "ci running" },
  failed: { state: "failing", says: "ci failed" },
  waiting_on_main: { state: "blocked", says: "ci red because main is" },
};

export type PullBlock = {
  number: number;
  url: string;
  branch: string;
  state: PullState;
  /** In the forge's merge queue now, rather than waiting to join it. */
  queued: boolean;
  /** The registry's glyph and status token for the state. Absent where nothing has run. */
  mark: NonNullable<(typeof MARK)[PullState]> | undefined;
  owner: Owner | undefined;
  /** Title, branch, state and owner, as the hover says them. */
  tip: string;
};

/** The pull requests of one repository's line, in the order the strip draws them, nearest main first. */
export function pullBlocks(
  view: Pick<MergeLineView, "root" | "hub">,
  lines: MergeLines | null,
  views: readonly Pick<MergeLineView, "root" | "hub">[],
  sessions: readonly Session[],
): readonly PullBlock[] {
  const wire = lines?.lines.find((one) => one.root === view.root)?.hub?.pull_requests ?? [];
  return (view.hub?.pulls ?? []).map((pull) => {
    const reading = pull.queue === undefined ? (pull.ci === undefined ? undefined : CI[pull.ci]) : QUEUE[pull.queue.state];
    const state = reading?.state ?? "open";
    const owner = pullOwner(views, sessions, pull.number, pull.branch);
    const title = wire.find((one) => one.number === pull.number)?.title;
    const position = pull.queue?.position;
    const tip = [
      `#${pull.number}${title === undefined || title === "" ? "" : ` ${title}`}`,
      pull.branch,
      `${reading?.says ?? "Open"}${position === undefined ? "" : `, place ${position}`}`,
      ...(owner === undefined ? [] : [`${owner.kind === "job" ? "Job" : "Session"}: ${owner.title}`]),
    ].join(" · ");
    return {
      number: pull.number,
      url: pull.url,
      branch: pull.branch,
      state,
      queued: pull.queue !== undefined && pull.queue.state !== "waiting_for_ci",
      mark: MARK[state],
      owner,
      tip,
    };
  });
}

/** What a chip does when pressed: the pull request on the forge, or the Job that owns the entry. */
export type ChipAct = { kind: "link"; url: string } | { kind: "job"; id: string };

/** One thing on the strip, a pull request or a landing, drawn the same way. */
export type Chip = {
  key: string;
  label: string;
  state: PullState;
  /** Held by a queue, Fleet's or the forge's, rather than open and waiting. Drawn solid. */
  queued: boolean;
  icon: LucideIcon | null;
  tip: string;
  act: ChipAct | undefined;
};

/** The last path segment of a branch, a `worktree-agent-<hash>` as `agent-` and the hash's first four, at most 12 characters. */
export function shortBranch(branch: string): string {
  const last = branch.split("/").pop() ?? branch;
  const agent = /^worktree-agent-([0-9a-f]{4})[0-9a-f]*$/i.exec(last);
  const name = agent === null ? last : `agent-${agent[1]}`;
  return name.length > 12 ? `${name.slice(0, 11)}…` : name;
}

/** A landing's stage on the state colours: running its turn is running, a red or a conflict is failing. */
const LANDING: Record<string, PullState> = {
  waiting: "queued",
  preparing: "running",
  gating: "running",
  merging: "running",
  held: "blocked",
  landed: "ready",
  red: "failing",
  conflict: "failing",
  stopped: "failing",
};

/** Fleet's landing entries, in line order, as chips. */
export function landingChips(line: MergeLineView["line"]): readonly Chip[] {
  return line.map((entry) => {
    const stage = LAND_STATE[entry.state]?.verb ?? entry.state;
    const act: ChipAct | undefined = entry.pr !== undefined ? { kind: "link", url: entry.pr.url } : entry.job !== undefined ? { kind: "job", id: entry.job.id } : undefined;
    return {
      key: `line:${entry.branch}`,
      label: shortBranch(entry.branch),
      state: LANDING[entry.state] ?? "open",
      queued: true,
      icon: GitMerge,
      tip: [entry.branch, stage, ...(entry.job === undefined ? [] : [`Job: ${entry.job.title}`])].join(" · "),
      act,
    };
  });
}

/** Every chip of one repository's strip, nearest main first. */
export function chipsOf(
  view: Pick<MergeLineView, "root" | "hub" | "line">,
  lines: MergeLines | null,
  views: readonly Pick<MergeLineView, "root" | "hub">[],
  sessions: readonly Session[],
): readonly Chip[] {
  return [
    ...landingChips(view.line),
    ...pullBlocks(view, lines, views, sessions).map(
      (pull): Chip => ({
        key: `pull:${pull.number}`,
        label: `#${pull.number}`,
        state: pull.state,
        queued: pull.queued,
        icon: pull.mark?.icon ?? null,
        tip: pull.tip,
        act: { kind: "link", url: pull.url },
      }),
    ),
  ];
}
