// What a Session has built, read off what Fleet recorded: the phase it is in, one piece for each task of its list, the survey markers,
// the subagents at work, the quiet since its last tool call and the road from its structure to main. Pure; nothing here draws.
//
// **Every figure is derived, none is held.** Reads, edits, subagents, questions and pull request stages are timeline entries Fleet
// already writes. Two facts have no entry yet, so a view carries them beside the events: the task list with the time each task
// was ticked (the TodoWrite hook), and a walk shown (`show_window`). A tick with an edit burst since the previous tick is solid;
// a tick with none is hollow, claimed and not seen. #2301.

import { BadgeCheck, Eye, Footprints, GitMerge, GitPullRequest, Layers, ListTodo, Scale, ScanSearch, ShieldCheck, type LucideIcon } from "lucide-react";
import type { JobDetail, TimelineEvent } from "@armada/protocol";
import { taskViewsOf, type TaskState } from "@armada/jobs/draft/task";

import { FRESH_MS, doingOfEvent, type Doing } from "./doing";
import type { DroneIn, Point, Rect } from "./layout";

/** How literal the construction is: a building, a launch gantry with a craft, or a ring on the ground. */
export type Take = "tower" | "pad" | "ring";
/** What a landed structure does: stands small and weathered for a day, goes on landing, or stays. */
export type After = "stand" | "clear" | "monument";
/** Where the ring take lays the road after Implement: a thinner ring round the task ring, or one inside it. */
export type RoadAt = "outer" | "inner";

/** One task of the list the Session wrote. `ticked` is when it marked it done. */
export type Task = { id: string; text: string; planned: string; ticked?: string };

/** Everything the field needs of one Session's construction, at the clock `now`. */
export type ProgressView = {
  /** The drone it is, or was: `session:<id>`. */
  key: string;
  root: string;
  /** The directory it writes most. A drone that is on the map stands over its own plot and this is only the fallback. */
  tile: string;
  /** Its turn is running. */
  running: boolean;
  now: number;
  events: readonly TimelineEvent[];
  tasks: readonly Task[];
  /** Where on the plot a structure with no drone stands, 0 to 8. */
  slot?: number;
  /** The stops of its road, where it is not a Session's own. */
  road?: readonly StopId[];
};
export type ProgressFeed = { take: Take; after: After; road?: RoadAt; views: readonly ProgressView[] };

export type Piece = "ghost" | "scaffold" | "hollow" | "solid";
export type Phase = "scope" | "plan" | "implement" | "road" | "landed";
/** A Session's road is a walk, a pull request, its Checks and the auto-merge; a Job's is the steps of its workflow after Implement. */
export type StopId = "walk" | "pr" | "checks" | "auto" | "check" | "judge" | "review" | "land";
export type Stop = { id: StopId; at: number | undefined };

export type Built = {
  key: string;
  root: string;
  tile: string;
  phase: Phase;
  /** The stop most recently reached, once the road is laid. */
  reached: StopId | undefined;
  tasks: readonly { text: string; piece: Piece; /** What a hover says of it where "ticked, edits seen" is not the word: a Job's task is done or handed in. */ said?: string }[];
  /** The walks shown and questions asked before the build was done, in order. */
  markers: readonly ("walk" | "asked")[];
  /** The subagents running now, by what each was asked. */
  workers: readonly string[];
  /** Minutes since the last tool call, while the turn runs. */
  quiet: number;
  stalled: boolean;
  /** How far the unfinished part has dimmed, 0 to 1. */
  dim: number;
  doing: Doing | undefined;
  via: string | undefined;
  stops: readonly Stop[];
  landedAt: number | undefined;
  now: number;
  slot: number | undefined;
};

const MIN = 60_000;
/** A running turn quiet this long has stalled. */
export const STALL_MIN = 10;
/** The unfinished part dims from the stall to this many minutes, and no further. */
const DIM_FULL_MIN = 40;

const at = (event: TimelineEvent) => Date.parse(event.at);
/** Reads, edits, runs, questions and the Session's own turns: what a quiet turn has not done. Subagent entries are theirs. */
const OWN = new Set<TimelineEvent["kind"]>(["session.read", "session.edited", "session.ran", "session.asked", "session.turn", "session.waiting"]);
const IS_WALK = (event: TimelineEvent) => event.kind === "session.waiting" && event.says === "Walk shown";

export const PHASE_WORD: Record<Phase, string> = { scope: "Scope", plan: "Plan", implement: "Implement", road: "Road", landed: "Landed" };
export const PHASE_GLYPH: Record<Phase, LucideIcon> = { scope: ScanSearch, plan: ListTodo, implement: Layers, road: GitPullRequest, landed: BadgeCheck };
export const STOP_WORD: Record<StopId, string> = { walk: "Walk shown", pr: "Pull request opened", checks: "Checks running", auto: "Auto-merge set", check: "Check", judge: "Judge", review: "Review", land: "Land" };
export const STOP_GLYPH: Record<StopId, LucideIcon> = { walk: Footprints, pr: GitPullRequest, checks: ShieldCheck, auto: GitMerge, check: ShieldCheck, judge: Scale, review: Eye, land: GitMerge };
export const STOPS: readonly StopId[] = ["walk", "pr", "checks", "auto"];

/** The unfinished part's dimming from how long the turn has been quiet: nothing before the stall, 0.3 at it, 0.65 at forty minutes. */
export const dimOf = (quiet: number): number => (quiet < STALL_MIN ? 0 : 0.3 + 0.35 * Math.min(1, (quiet - STALL_MIN) / (DIM_FULL_MIN - STALL_MIN)));

export function builtOf(view: ProgressView): Built {
  const { now } = view;
  const seen = view.events.filter((event) => at(event) <= now).sort((a, b) => at(a) - at(b));
  const planned = view.tasks.filter((task) => Date.parse(task.planned) <= now);
  const edits = seen.filter((event) => event.kind === "session.edited" || event.kind === "job.files").map(at);
  const ticks = planned.flatMap((task) => (task.ticked !== undefined && Date.parse(task.ticked) <= now ? [Date.parse(task.ticked)] : [])).sort((a, b) => a - b);
  const since = planned.length === 0 ? now : Math.min(...planned.map((task) => Date.parse(task.planned)));
  const before = (tick: number) => ticks.filter((one) => one < tick).at(-1) ?? since;
  const edited = (from: number, to: number) => edits.some((one) => one > from && one <= to);

  const pieces: Piece[] = planned.map((task) => {
    const tick = task.ticked === undefined ? undefined : Date.parse(task.ticked);
    return tick === undefined || tick > now ? "ghost" : edited(before(tick), tick) ? "solid" : "hollow";
  });
  // Edits with no tick since the last one: scaffold goes up round the next ghost piece, so the structure is never still while it is worked on.
  const next = pieces.indexOf("ghost");
  if (next >= 0 && edited(ticks.at(-1) ?? since, now)) pieces[next] = "scaffold";

  const done = planned.length > 0 && !pieces.includes("ghost") && !pieces.includes("scaffold");
  const doneAt = done ? (ticks.at(-1) ?? now) : Infinity;
  const markers = seen.flatMap((event): ("walk" | "asked")[] => (at(event) >= doneAt ? [] : IS_WALK(event) ? ["walk"] : event.kind === "session.asked" ? ["asked"] : []));

  const started = new Map<string, number>();
  for (const event of seen) {
    if (event.kind !== "session.subagent" || event.detail === undefined) continue;
    if (event.says === "Subagent started") started.set(event.detail, (started.get(event.detail) ?? 0) + 1);
    else started.set(event.detail, (started.get(event.detail) ?? 0) - 1);
  }
  const workers = [...started].filter(([, n]) => n > 0).map(([detail]) => detail);

  const found = (test: (event: TimelineEvent) => boolean, after = -Infinity) => seen.find((event) => at(event) >= after && test(event));
  const whenFound = (test: (event: TimelineEvent) => boolean, after?: number) => {
    const event = found(test, after);
    return event === undefined ? undefined : at(event);
  };
  const landed = found((event) => event.kind === "pr.merged" || event.kind === "job.landed");
  // The road's walk is one of the finished thing: a walk shown while the build was still going is a survey marker.
  const stopAt = (id: StopId) =>
    id === "walk" ? (done ? whenFound(IS_WALK, doneAt) : undefined)
    : id === "pr" ? whenFound((event) => event.kind === "pr.opened")
    : id === "checks" ? whenFound((event) => event.kind === "pr.checks")
    : id === "auto" ? whenFound((event) => event.kind === "pr.queued")
    : whenFound((event) => event.kind === "job.step" && event.detail === id);
  const stops: Stop[] = (view.road ?? STOPS).map((id) => ({ id, at: stopAt(id) }));
  const reached = stops.filter((stop) => stop.at !== undefined).sort((a, b) => b.at! - a.at!)[0]?.id;

  const own = seen.filter((event) => OWN.has(event.kind));
  const last = Math.max(...own.map(at), ...ticks, -Infinity);
  const quiet = view.running && landed === undefined && Number.isFinite(last) ? Math.max(0, Math.floor((now - last) / MIN)) : 0;
  const stalled = quiet >= STALL_MIN;
  const tool = [...seen].reverse().find((event) => doingOfEvent(event) !== undefined);
  const fresh = tool !== undefined && view.running && at(tool) + FRESH_MS > now ? tool : undefined;

  const phase: Phase = landed !== undefined ? "landed" : done || stops.some((stop) => stop.at !== undefined) ? "road" : edits.length > 0 ? "implement" : planned.length > 0 ? "plan" : "scope";
  return {
    key: view.key,
    root: view.root,
    tile: view.tile,
    phase,
    reached,
    tasks: planned.map((task, i) => ({ text: task.text, piece: pieces[i]! })),
    markers,
    workers,
    quiet,
    stalled,
    dim: stalled ? dimOf(quiet) : 0,
    doing: fresh === undefined ? undefined : doingOfEvent(fresh),
    via: fresh?.detail,
    stops,
    landedAt: landed === undefined ? undefined : at(landed),
    now,
    slot: view.slot,
  };
}

const PLAN_PIECE: Record<TaskState, { piece: Piece; said: string } | undefined> = {
  done: { piece: "solid", said: "Done" },
  handed_in: { piece: "hollow", said: "Handed in" },
  working: { piece: "scaffold", said: "Working" },
  open: { piece: "ghost", said: "Open" },
  failed: { piece: "ghost", said: "Failed" },
  dropped: undefined,
};

/**
 * A Job's structure on its route: one piece for each task of the plan Fleet holds, as the plan says it stands (done is solid, handed in
 * is hollow, working is scaffold, open or failed is a ghost; a dropped task is no part of it). Nothing is invented: the Session's two
 * unrecorded facts are not needed, since the plan carries each task's state. `key` is the Job's drone, so its laser finds the structure.
 */
export function jobBuiltOf(whole: JobDetail, key: string, root: string, doing: Doing | undefined, now: number): Built | undefined {
  const tasks = taskViewsOf(whole).flatMap((task) => {
    const read = PLAN_PIECE[task.state];
    return read === undefined ? [] : [{ text: task.title, ...read }];
  });
  if (tasks.length === 0) return undefined;
  return { key, root, tile: "", phase: "implement", reached: undefined, tasks, markers: [], workers: [], quiet: 0, stalled: false, dim: 0, doing, via: undefined, stops: [], landedAt: undefined, now, slot: undefined };
}

/** One arc of the ring's road: a stop, or the landing that closes the circle. */
export type Arc = { id: StopId | "landed"; at: number | undefined };
export const arcsOf = (built: Built): readonly Arc[] => [...built.stops.map((stop) => ({ id: stop.id, at: stop.at })), { id: "landed", at: built.landedAt }];
export const arcWord = (id: Arc["id"]): string => (id === "landed" ? PHASE_WORD.landed : STOP_WORD[id]);
export const arcSaid = (arc: Arc, now: number): string => (arc.at === undefined ? "Usually next on this repo" : agoOf(now - arc.at));

/** A Session's construction as its card tells it: the phase, how long it has been quiet, each task as its piece stands, and the road's stops (`ago` absent is one not yet reached). */
export type BuildTip = {
  phase: { Glyph: LucideIcon; word: string };
  quiet?: string;
  tasks: readonly { text: string; piece: Piece }[];
  stops: readonly { Glyph: LucideIcon; word: string; ago?: string }[];
};

/** How long ago, as a card says it. */
export const agoOf = (ms: number): string => {
  const minutes = Math.max(0, Math.round(ms / MIN));
  return minutes < 60 ? `${minutes} min ago` : minutes < 60 * 24 ? `${Math.round(minutes / 60)} h ago` : `${Math.round(minutes / 60 / 24)} d ago`;
};

/** The card's reading of a build. The road's stops are listed once the road is laid, each with when it happened or none where it has not. */
export function buildTipOf(built: Built): BuildTip {
  const roaded = built.phase === "road" || built.phase === "landed";
  const stop = (Glyph: LucideIcon, word: string, when: number | undefined) => ({ Glyph, word, ...(when === undefined ? {} : { ago: agoOf(built.now - when) }) });
  return {
    phase: { Glyph: glyphOf(built), word: wordOf(built) },
    ...(built.stalled ? { quiet: `Quiet ${built.quiet} min` } : {}),
    tasks: built.tasks,
    stops: roaded ? [...built.stops.map((one) => stop(STOP_GLYPH[one.id], STOP_WORD[one.id], one.at)), stop(PHASE_GLYPH.landed, PHASE_WORD.landed, built.landedAt)] : [],
  };
}

/** The drone as its construction says it stands: a stalled one hangs still under its amber ring, one with a fresh tool call does what that call says, the rest do nothing. */
export function shapedBy(drone: DroneIn, built: Built): DroneIn {
  const { doing: _doing, check: _check, via: _via, ...rest } = drone;
  if (built.stalled) return { ...rest, stuck: true, spin: false, hover: true, hue: "ask" };
  return built.doing === undefined ? rest : { ...rest, doing: built.doing, check: "run", ...(built.via === undefined ? {} : { via: built.via }) };
}

/** The word the card gives the phase: the stop last reached once the road is laid, else the phase. */
export const wordOf = (built: Built): string => (built.phase === "road" && built.reached !== undefined ? STOP_WORD[built.reached] : PHASE_WORD[built.phase]);
export const glyphOf = (built: Built): LucideIcon => (built.phase === "road" && built.reached !== undefined ? STOP_GLYPH[built.reached] : PHASE_GLYPH[built.phase]);

// ---- Where it stands on its plot ---------------------------------------------------------------------

/** Fractions of a plot where a structure with no drone to stand beside is put, so a small city on one slab does not stack. */
const SLOTS: readonly Point[] = [
  { x: 0.2, y: 0.34 },
  { x: 0.8, y: 0.34 },
  { x: 0.2, y: 0.8 },
  { x: 0.8, y: 0.8 },
  { x: 0.5, y: 0.22 },
  { x: 0.5, y: 0.88 },
  { x: 0.35, y: 0.58 },
  { x: 0.65, y: 0.58 },
  { x: 0.08, y: 0.58 },
];

/**
 * Where a structure stands: beside its drone, which hovers over the plot's middle, or at a place on the plot where none is. A ring is
 * round the drone's own shadow. Kept inside the plot.
 */
export function anchorOf(take: Take, rect: Rect, drone: Point | undefined, slot: number | undefined, big: number): Point {
  const clamp = (n: number, lo: number, hi: number) => (lo > hi ? (lo + hi) / 2 : Math.min(hi, Math.max(lo, n)));
  const from = drone === undefined ? { x: rect.x + rect.w * SLOTS[(slot ?? 0) % SLOTS.length]!.x, y: rect.y + rect.h * SLOTS[(slot ?? 0) % SLOTS.length]!.y } : take === "ring" ? drone : { x: drone.x + 58 * big, y: drone.y + 8 };
  return { x: clamp(from.x, rect.x + 34, rect.x + rect.w - 34), y: clamp(from.y, rect.y + 24, rect.y + rect.h - 6) };
}
