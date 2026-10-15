// What a Session or a Job has built, read for the field: the phase it is in, one piece for each task, the stops of the road, the
// subagents at work and how long it has sat quiet. Pure; nothing here draws.
//
// **Every figure is Fleet's.** A Session's `build` is derived by Fleet from what it observed and carried on its record
// (`SessionRecord.build`); this file only reads it, and adds nothing the record does not say. A Job's pieces are the states of the
// tasks of its plan. A Session with no `build` has no ring.

import { BadgeCheck, Footprints, GitCommitHorizontal, GitMerge, GitPullRequest, Layers, ListTodo, ScanSearch, ShieldCheck, type LucideIcon } from "lucide-react";
import type { BuildPhase, BuildPieceKind, BuildStopId, JobDetail, SessionBuild, TaskCounts, TimelineEvent } from "@armada/protocol";
import { taskViewsOf, type TaskState } from "@armada/jobs/draft/task";

import type { DroneIn } from "./layout";

export type Piece = BuildPieceKind;
export type Phase = BuildPhase;
export type StopId = BuildStopId;
/** A stop of the road. `at` is absent for one not reached: a forecast, drawn faint. */
export type Stop = { id: StopId; at: number | undefined };

/** One task of the ring. `said` is what a hover says of it where the piece's own word is not the fact: a Job's task is done or handed in. */
export type Task = { text: string; piece: Piece; said?: string };

/** Everything the field needs of one construction, at the clock `now`. */
export type BuildIn = {
  /** The drone it is the work of: `session:<id>` or `job:<id>`. */
  key: string;
  phase: Phase;
  /** The stop most recently reached, once the road is laid. */
  reached: StopId | undefined;
  tasks: readonly Task[];
  /** A walk shown or a question asked while it was still being scoped, in order. */
  markers: readonly ("walk" | "asked")[];
  /** The subagents running now, each as Fleet words it: `Type: what it was asked`. */
  workers: readonly string[];
  stalled: boolean;
  /** Minutes the unfinished part has sat quiet, while stalled. */
  quiet: number;
  /** How far the unfinished part has dimmed, 0 to 1. */
  dim: number;
  stops: readonly Stop[];
  landedAt: number | undefined;
  now: number;
};

const MIN = 60_000;
/** A turn quiet this long has stalled. */
export const STALL_MIN = 10;
/** The unfinished part dims from the stall to this many minutes, and no further. */
const DIM_FULL_MIN = 40;

export const PHASE_WORD: Record<Phase, string> = { scope: "Scope", plan: "Plan", implement: "Implement", road: "Road", landed: "Landed" };
export const PHASE_GLYPH: Record<Phase, LucideIcon> = { scope: ScanSearch, plan: ListTodo, implement: Layers, road: GitPullRequest, landed: BadgeCheck };
export const STOP_WORD: Record<StopId, string> = { walk: "Walk shown", pushed: "Pushed", pr: "Pull request opened", checks: "Checks", auto_merge: "Auto-merge" };
export const STOP_GLYPH: Record<StopId, LucideIcon> = { walk: Footprints, pushed: GitCommitHorizontal, pr: GitPullRequest, checks: ShieldCheck, auto_merge: GitMerge };

/** The unfinished part's dimming from how long the turn has been quiet: nothing before the stall, 0.3 at it, 0.65 at forty minutes. */
export const dimOf = (quiet: number): number => (quiet < STALL_MIN ? 0 : 0.3 + 0.35 * Math.min(1, (quiet - STALL_MIN) / (DIM_FULL_MIN - STALL_MIN)));

const at = (iso: string | undefined): number | undefined => (iso === undefined ? undefined : Date.parse(iso));

/** What a stall dims the ring by: how long since Fleet last saw it work. A stall with no instant held is as long as the stall itself. */
export function stallOf(build: Pick<SessionBuild, "stalled" | "quiet_since">, now: number): { stalled: boolean; quiet: number; dim: number } {
  if (!build.stalled) return { stalled: false, quiet: 0, dim: 0 };
  const since = at(build.quiet_since);
  const quiet = Math.max(STALL_MIN, since === undefined ? 0 : Math.floor((now - since) / MIN));
  return { stalled: true, quiet, dim: dimOf(quiet) };
}

/** The walks shown and questions asked by a Session: the stakes on its survey. */
export function markersOf(events: readonly TimelineEvent[], id: string): ("walk" | "asked")[] {
  return events
    .filter((one) => one.subject.kind === "session" && one.subject.id === id && (one.kind === "session.walked" || one.kind === "session.asked"))
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at))
    .map((one) => (one.kind === "session.walked" ? "walk" : "asked"));
}

/** A Session's ring, from the `build` on its record. `events` are the timeline's, which carry its survey markers while it is scoping. */
export function buildInOf(id: string, build: SessionBuild, now: number, events: readonly TimelineEvent[] = []): BuildIn {
  const stops = build.stops.map((stop): Stop => ({ id: stop.id, at: at(stop.at) }));
  const reached = stops.filter((stop) => stop.at !== undefined).sort((a, b) => b.at! - a.at!)[0]?.id;
  return {
    key: `session:${id}`,
    phase: build.phase,
    reached,
    tasks: build.pieces.map((one) => ({ text: one.text, piece: one.piece })),
    markers: build.phase === "scope" ? markersOf(events, id) : [],
    workers: build.workers,
    ...stallOf(build, now),
    stops,
    landedAt: at(build.landed_at),
    now,
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
 * A Job's ring: one piece for each task of the plan Fleet holds, as the plan says it stands. A dropped task is no part of it. It has no
 * road of its own, since its route is the road. None where the Job has no plan.
 */
export function jobBuildOf(whole: JobDetail, key: string, now: number): BuildIn | undefined {
  const tasks = taskViewsOf(whole).flatMap((task): Task[] => {
    const read = PLAN_PIECE[task.state];
    return read === undefined ? [] : [{ text: task.title, ...read }];
  });
  if (tasks.length === 0) return undefined;
  return { key, phase: "implement", reached: undefined, tasks, markers: [], workers: [], stalled: false, quiet: 0, dim: 0, stops: [], landedAt: undefined, now };
}

/** The phase a Job's row shows: Implement while it has a plan with tasks in it, else none. */
export const jobPhaseOf = (tasks: TaskCounts | undefined): Phase | undefined =>
  tasks !== undefined && tasks.done + tasks.working + tasks.open + (tasks.handed_in ?? 0) + (tasks.failed ?? 0) > 0 ? "implement" : undefined;

/** One arc of the ring's road: a stop, or the landing that closes the circle. */
export type Arc = { id: StopId | "landed"; at: number | undefined };
export const arcsOf = (build: BuildIn): readonly Arc[] => [...build.stops.map((stop) => ({ id: stop.id, at: stop.at })), { id: "landed", at: build.landedAt }];
export const arcWord = (id: Arc["id"]): string => (id === "landed" ? PHASE_WORD.landed : STOP_WORD[id]);
export const arcSaid = (arc: Arc, now: number): string => (arc.at === undefined ? "Usually next on this repo" : agoOf(now - arc.at));

/** How long ago, as a card says it. */
export const agoOf = (ms: number): string => {
  const minutes = Math.max(0, Math.round(ms / MIN));
  return minutes < 60 ? `${minutes} min ago` : minutes < 60 * 24 ? `${Math.round(minutes / 60)} h ago` : `${Math.round(minutes / 60 / 24)} d ago`;
};

/** The word and glyph a phase is drawn with: the stop last reached once the road is laid, else the phase. */
export const wordOf = (build: Pick<BuildIn, "phase" | "reached">): string => (build.phase === "road" && build.reached !== undefined ? STOP_WORD[build.reached] : PHASE_WORD[build.phase]);
export const glyphOf = (build: Pick<BuildIn, "phase" | "reached">): LucideIcon => (build.phase === "road" && build.reached !== undefined ? STOP_GLYPH[build.reached] : PHASE_GLYPH[build.phase]);

/** A construction as its card tells it: the phase, how long it has been quiet, each task as its piece stands, and the road's stops (`ago` absent is one not yet reached). */
export type BuildTip = {
  phase: { Glyph: LucideIcon; word: string };
  quiet?: string;
  tasks: readonly { text: string; piece: Piece; said?: string }[];
  stops: readonly { Glyph: LucideIcon; word: string; ago?: string }[];
};

/** The card's reading of a build. The road's stops are listed once the road is laid, each with when it happened or none where it has not. */
export function buildTipOf(build: BuildIn): BuildTip {
  const roaded = build.phase === "road" || build.phase === "landed";
  const stop = (Glyph: LucideIcon, word: string, when: number | undefined) => ({ Glyph, word, ...(when === undefined ? {} : { ago: agoOf(build.now - when) }) });
  return {
    phase: { Glyph: glyphOf(build), word: wordOf(build) },
    ...(build.stalled ? { quiet: `Quiet ${build.quiet} min` } : {}),
    tasks: build.tasks,
    stops: roaded ? [...build.stops.map((one) => stop(STOP_GLYPH[one.id], STOP_WORD[one.id], one.at)), stop(PHASE_GLYPH.landed, PHASE_WORD.landed, build.landedAt)] : [],
  };
}

/** The drone as its build says it stands: a stalled one hangs still under its amber ring. Any other is as the field had it. */
export const shapedBy = (drone: DroneIn, build: BuildIn): DroneIn => (build.stalled ? { ...drone, stuck: true, spin: false, hover: true, hue: "ask" } : drone);
