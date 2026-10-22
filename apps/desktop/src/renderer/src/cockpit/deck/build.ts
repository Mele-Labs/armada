// What a Session has built, read for the field: the phase it is in, one piece for each task, the stops of the road, the
// subagents at work and how long it has sat quiet. Pure; nothing here draws.
//
// **Every figure is Fleet's.** A Session's `build` is derived by Fleet from what it observed and carried on its record
// (`SessionRecord.build`); this file only reads it, and adds nothing the record does not say.
// A Session with no `build` has no ring.

import { BadgeCheck, Footprints, GitCommitHorizontal, GitMerge, GitPullRequest, Layers, ListTodo, ScanSearch, ShieldCheck, type LucideIcon } from "lucide-react";
import type { BuildPhase, BuildPieceKind, BuildStopId, SessionBuild, TimelineEvent } from "@armada/protocol";

import { ageOf } from "@armada/components";
import { FRESH_MS, doingOfEvent } from "./doing";
import type { DroneIn } from "./layout";

export type Piece = BuildPieceKind;
export type Phase = BuildPhase;
export type StopId = BuildStopId;
/** A stop of the road. `at` is absent for one not reached: a forecast, drawn faint. */
export type Stop = { id: StopId; at: number | undefined };

/** One task of the ring. */
export type Task = { text: string; piece: Piece; /** The subagent that wrote it, as its worker label (`Type: what it was asked`). */ by?: string };

/** Everything the field needs of one construction, at the clock `now`. */
export type BuildIn = {
  /** The drone it is the work of: `session:<id>`. */
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
  /** The drone is writing now: its latest tool burst is an edit and recent. Moves the band that stands for a ring with no task list. */
  writing: boolean;
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

/** What a subagent was asked: its label without the type. */
export const askedOf = (label: string): string => label.replace(/^[^:]*: /, "");

/**
 * The tasks in the order the ring draws them: the Session's own keep list order, and each subagent's are drawn together as one stretch, the
 * stretches in the order each subagent's first task appears, each where that first task stood. Subagents share one list, so parallel ones
 * arrive interleaved. A finished subagent's stretch stays.
 */
export function groupedTasks<T extends { by?: string }>(tasks: readonly T[]): T[] {
  const out: T[] = [];
  const done = new Set<string>();
  for (const task of tasks) {
    if (task.by === undefined) out.push(task);
    else if (!done.has(task.by)) {
      done.add(task.by);
      out.push(...tasks.filter((one) => one.by === task.by));
    }
  }
  return out;
}

/**
 * Where a worker sits on the ring, in degrees as the ring's sectors are laid (from the top, clockwise): the middle of its stretch, the pieces it
 * wrote, which `groupedTasks` has made contiguous. None where it has written none.
 */
export function seatOf(tasks: readonly Pick<Task, "by">[], label: string): number | undefined {
  const mine = tasks.flatMap((task, k) => (task.by === label ? [k] : []));
  if (mine.length === 0) return undefined;
  const mid = (k: number) => -90 + ((k + 0.5) * 360) / tasks.length;
  return (mid(mine[0]!) + mid(mine[mine.length - 1]!)) / 2;
}

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

/** Whether a Session is writing: its latest tool burst is an edit, and it is still the present. */
export function writingOf(events: readonly TimelineEvent[], id: string, now: number): boolean {
  let latest: TimelineEvent | undefined;
  for (const one of events) {
    if (one.subject.kind !== "session" || one.subject.id !== id || doingOfEvent(one) === undefined || Date.parse(one.at) > now) continue;
    if (latest === undefined || Date.parse(one.at) >= Date.parse(latest.at)) latest = one;
  }
  return latest !== undefined && doingOfEvent(latest) === "write" && Date.parse(latest.at) + FRESH_MS > now;
}

/** A Session's ring, from the `build` on its record. `events` are the timeline's, which carry its survey markers while it is scoping. */
export function buildInOf(id: string, build: SessionBuild, now: number, events: readonly TimelineEvent[] = []): BuildIn {
  const stops = build.stops.map((stop): Stop => ({ id: stop.id, at: at(stop.at) }));
  const reached = stops.filter((stop) => stop.at !== undefined).sort((a, b) => b.at! - a.at!)[0]?.id;
  const stall = stallOf(build, now);
  return {
    key: `session:${id}`,
    phase: build.phase,
    reached,
    tasks: groupedTasks(build.pieces.map((one): Task => ({ text: one.text, piece: one.piece, ...(one.by === undefined ? {} : { by: one.by }) }))),
    markers: build.phase === "scope" ? markersOf(events, id) : [],
    workers: build.workers,
    ...stall,
    writing: !stall.stalled && writingOf(events, id, now),
    stops,
    landedAt: at(build.landed_at),
    now,
  };
}

/** One arc of the ring's road: a stop, or the landing that closes the circle. */
export type Arc = { id: StopId | "landed"; at: number | undefined };
export const arcsOf = (build: BuildIn): readonly Arc[] => [...build.stops.map((stop) => ({ id: stop.id, at: stop.at })), { id: "landed", at: build.landedAt }];
/** The road is drawn in any phase once it has a stop, or the landing. */
export const roadOf = (build: BuildIn): readonly Arc[] | undefined => (build.stops.length === 0 && build.landedAt === undefined ? undefined : arcsOf(build));
export const arcWord = (id: Arc["id"]): string => (id === "landed" ? PHASE_WORD.landed : STOP_WORD[id]);
export const arcSaid = (arc: Arc, now: number): string => (arc.at === undefined ? "Usually next on this repo" : ageOf(arc.at, now));

/**
 * What stands for the task ring where the agent wrote no list: one unsegmented band, since without a list nothing says how much is
 * left and a partial fill would claim a fraction nobody measured. A faint outline while scoping or planning, an even scaffold once
 * edits are seen and a solid band when landed. `moving` only while the drone is writing. None where there are tasks.
 */
export type Band = { state: "outline" | "scaffold" | "solid"; moving: boolean; word: string };
export function bandOf(build: BuildIn): Band | undefined {
  if (build.tasks.length > 0) return undefined;
  const word = PHASE_WORD[build.phase];
  if (build.phase === "landed" || build.landedAt !== undefined) return { state: "solid", moving: false, word };
  if (build.phase === "implement" || build.phase === "road") return { state: "scaffold", moving: build.writing, word };
  return { state: "outline", moving: false, word };
}

/** The word and glyph a phase is drawn with: the stop last reached once the road is laid, else the phase. */
export const wordOf = (build: Pick<BuildIn, "phase" | "reached">): string => (build.phase === "road" && build.reached !== undefined ? STOP_WORD[build.reached] : PHASE_WORD[build.phase]);
export const glyphOf = (build: Pick<BuildIn, "phase" | "reached">): LucideIcon => (build.phase === "road" && build.reached !== undefined ? STOP_GLYPH[build.reached] : PHASE_GLYPH[build.phase]);

/** A construction as its card tells it: the phase, how long it has been quiet, each task as its piece stands, and the road's stops (`ago` absent is one not yet reached). */
export type BuildTip = {
  phase: { Glyph: LucideIcon; word: string };
  quiet?: string;
  tasks: readonly { text: string; piece: Piece }[];
  stops: readonly { Glyph: LucideIcon; word: string; ago?: string }[];
};

/** The card's reading of a build. The road's stops are listed as the ring draws them, once it has any, each with when it happened or none where it has not. */
export function buildTipOf(build: BuildIn): BuildTip {
  const roaded = roadOf(build) !== undefined;
  const stop = (Glyph: LucideIcon, word: string, when: number | undefined) => ({ Glyph, word, ...(when === undefined ? {} : { ago: ageOf(when, build.now) }) });
  return {
    phase: { Glyph: glyphOf(build), word: wordOf(build) },
    ...(build.stalled ? { quiet: `Quiet ${build.quiet} min` } : {}),
    tasks: build.tasks,
    stops: roaded ? [...build.stops.map((one) => stop(STOP_GLYPH[one.id], STOP_WORD[one.id], one.at)), stop(PHASE_GLYPH.landed, PHASE_WORD.landed, build.landedAt)] : [],
  };
}

/** The drone as its build says it stands: a stalled one hangs still under its amber ring. Any other is as the field had it. */
export const shapedBy = (drone: DroneIn, build: BuildIn): DroneIn => (build.stalled ? { ...drone, stuck: true, spin: false, hover: true, hue: "ask" } : drone);
