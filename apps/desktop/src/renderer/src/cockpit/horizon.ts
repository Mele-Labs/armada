// The Cockpit's horizon as one row of dots, nearest main first: Fleet's own landing entries
// (`view.line`), then the pull requests in the forge's merge queue by position, then the rest as the
// line lists them. The pull requests are read off the view's hub (`view.hub.pulls`, which the Merge
// line surface draws) with the title and the owner beside, so the strip folds nothing the surface
// does not. Pure, so the order and the marks are checked without drawing the band.

import type { LucideIcon } from "lucide-react";

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
  /** What the forge calls it. Absent where the wire carries none. */
  title: string | undefined;
  /** The queue's place, 1 being next to merge. */
  place: number | undefined;
  /** What the forge's checks say, where the queue's own word stands for the state. */
  check: string | undefined;
  says: string;
  /** Title, branch, state and owner, as the dot's name says them. */
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
      title: title === undefined || title === "" ? undefined : title,
      place: position,
      check: pull.queue !== undefined && pull.ci !== undefined ? CI[pull.ci]?.says : undefined,
      says: reading?.says ?? "Open",
      tip,
    };
  });
}

/** What a dot does when pressed: the pull request on the forge, or the Job that owns the entry. */
export type DotAct = { kind: "link"; url: string } | { kind: "job"; id: string };

/** What the dot's card says, row by row. A row is drawn only where its fact is. */
export type DotCard = {
  /** `#1890` for a pull request, the full branch for a landing. */
  heading: string;
  /** A pull request's title as the forge holds it. */
  title: string | undefined;
  branch: string;
  /** The state in words: the queue's or the ci's for a pull request, the registry's stage for a landing. */
  says: string;
  /** The checks: where ci stands inside the queue, a landing's failed or running Check. */
  check: string | undefined;
  place: number | undefined;
  owner: Owner | undefined;
};

/** One thing on the strip, a pull request or a landing, drawn the same way: a dot. */
export type Dot = {
  key: string;
  state: PullState;
  /** Held by a queue, Fleet's or the forge's, rather than open and waiting. Drawn solid, the rest a ring. */
  queued: boolean;
  /** The registry's glyph for the state, drawn in the card. Absent where nothing has run. */
  icon: LucideIcon | null;
  /** The dot's accessible name: the card's rows on one line. */
  tip: string;
  card: DotCard;
  act: DotAct | undefined;
};

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

/** A landing's Checks in a line: what failed, else what is running, else what Fleet says it is doing. */
function checkOf(entry: MergeLineView["line"][number]): string | undefined {
  if (entry.failed !== undefined && entry.failed.length > 0) return `Failed: ${entry.failed.join(", ")}`;
  const running = entry.checks?.find((one) => one.state === "running");
  if (running !== undefined) return `Running: ${running.name}`;
  return entry.doing;
}

/** Fleet's landing entries, in line order, as dots. */
export function landingDots(line: MergeLineView["line"]): readonly Dot[] {
  return line.map((entry) => {
    const says = LAND_STATE[entry.state]?.verb ?? entry.state;
    const owner: Owner | undefined = entry.job === undefined ? undefined : { kind: "job", id: entry.job.id, title: entry.job.title };
    const act: DotAct | undefined = entry.pr !== undefined ? { kind: "link", url: entry.pr.url } : entry.job !== undefined ? { kind: "job", id: entry.job.id } : undefined;
    const state = LANDING[entry.state] ?? "open";
    return {
      key: `line:${entry.branch}`,
      state,
      queued: true,
      icon: MARK[state]?.icon ?? null,
      tip: [entry.branch, says, ...(owner === undefined ? [] : [`Job: ${owner.title}`])].join(" · "),
      card: { heading: entry.branch, title: undefined, branch: entry.branch, says, check: checkOf(entry), place: entry.place, owner },
      act,
    };
  });
}

/** Every dot of one repository's strip, nearest main first. */
export function dotsOf(
  view: Pick<MergeLineView, "root" | "hub" | "line">,
  lines: MergeLines | null,
  views: readonly Pick<MergeLineView, "root" | "hub">[],
  sessions: readonly Session[],
): readonly Dot[] {
  return [
    ...landingDots(view.line),
    ...pullBlocks(view, lines, views, sessions).map(
      (pull): Dot => ({
        key: `pull:${pull.number}`,
        state: pull.state,
        queued: pull.queued,
        icon: pull.mark?.icon ?? null,
        tip: pull.tip,
        card: { heading: `#${pull.number}`, title: pull.title, branch: pull.branch, says: pull.says, check: pull.check, place: pull.place, owner: pull.owner },
        act: { kind: "link", url: pull.url },
      }),
    ),
  ];
}
