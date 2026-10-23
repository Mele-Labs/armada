// The Artifacts layer's timeline, as data: every artifact of a Job is a line that begins where it first appeared, and each later version of
// it is a point along that line. A line is born off another where a join is known (a pull request that closes an issue, a sketch redrawn
// against another, a child Job started from a plan group) and off the Job's own line where none is. It is read from the same rows the list
// draws (`artifacts.ts`) and from `Session.versions`, so the list's search narrows it and a press does what the list's row does. Commands,
// subagents and background tasks are not artifacts and are not on it. Where Fleet sent no history the points keep the order the Job holds
// its attachments in, one to each, and the axis has no clock.

import { Check, GitMerge, ShieldCheck, ShieldEllipsis, ShieldX, type LucideIcon } from "lucide-react";
import type { SessionLedgerTone } from "@armada/components";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import { shownOnly } from "../session-cards";
import type { ArtifactGroup, ArtifactRow } from "./artifacts";

export type LaneId = "job" | "link" | "issue" | "pull_request" | "sketch" | "page" | "file" | "doc";

/** The lanes in the order the second take stacks them. */
export const LANES: readonly { id: LaneId; label: string }[] = [
  { id: "job", label: "Jobs" },
  { id: "link", label: "Links" },
  { id: "issue", label: "Issues" },
  { id: "pull_request", label: "Pull requests" },
  { id: "sketch", label: "Sketches" },
  { id: "page", label: "Pages" },
  { id: "file", label: "Files" },
  { id: "doc", label: "Docs" },
];

export type TimelinePoint = {
  key: string;
  /** Milliseconds when the version carries an instant, else its place in the order the Job holds them. */
  t: number;
  /** What happened at it: `Opened`, `Checks failed`, `Redrawn`. */
  said: string;
  /** A state's own glyph and hue, for a version that changed one. */
  Glyph?: LucideIcon;
  tone?: SessionLedgerTone;
  open?: () => void;
};

export type TimelineLine = {
  key: string;
  lane: LaneId;
  Glyph: ArtifactRow["Glyph"];
  said: string;
  title: string;
  points: TimelinePoint[];
  /** The lines it branches off. `job` is the Job's own. Empty never: a line with no known join is born off the Job. */
  parents: { key: string; said?: string }[];
};

export type Timeline = { lines: TimelineLine[]; start: number; end: number; timed: boolean };

/** What opening a point may need that a row does not hold: the sketch as it stood at an instant. */
export type TimelineActs = { sketch: (id: string, at?: string) => void };

type Version = { at?: string; attachment: SessionAttachment };

const identityOf = (one: SessionAttachment): { key: string; lane: LaneId } | undefined => {
  switch (one.kind) {
    case "pull_request":
      return { key: `pr:${one.number}`, lane: "pull_request" };
    case "issue":
      return { key: `issue:${one.number}`, lane: "issue" };
    case "link":
      return { key: `link:${one.url}`, lane: "link" };
    case "sketch":
      return { key: `sketch:${one.id}`, lane: "sketch" };
    case "artifact":
      return { key: `artifact:${one.id}`, lane: one.form === "page" || one.form === "window" ? "page" : one.form === "doc" ? "doc" : "file" };
    case "started_to":
      return one.group === undefined ? undefined : { key: `session:${one.id}`, lane: "job" };
    default:
      return undefined;
  }
};

type What = { said: string; Glyph?: LucideIcon; tone?: SessionLedgerTone };

/** What a version did to the one before it, in the words the list's marks use. */
function whatOf(before: SessionAttachment | undefined, now: SessionAttachment, row: ArtifactRow): What {
  switch (now.kind) {
    case "pull_request": {
      if (before?.kind !== "pull_request") return { said: now.state === "draft" ? "Opened as draft" : "Opened" };
      if (now.state !== before.state) return now.state === "merged" ? { said: "Merged", Glyph: GitMerge, tone: "done" } : { said: now.state === "draft" ? "Back to draft" : "Ready for review" };
      if (now.checks.state !== before.checks.state) {
        return now.checks.state === "failed" ? { said: "Checks failed", Glyph: ShieldX, tone: "failed" } : now.checks.state === "pending" ? { said: "Checks running", Glyph: ShieldEllipsis, tone: "pending" } : { said: "Checks passed", Glyph: ShieldCheck, tone: "passed" };
      }
      return { said: "Changed" };
    }
    case "issue":
      if (before?.kind !== "issue") return { said: "Added" };
      return now.state === "closed" ? { said: "Closed", Glyph: Check, tone: "done" } : { said: "Reopened" };
    case "sketch":
      return { said: before === undefined ? "Drawn" : "Redrawn" };
    case "artifact":
      if (before === undefined) return { said: now.form === "page" || now.form === "doc" ? "Published" : now.form === "window" || shownOnly(now) || now.shown !== undefined ? "Shown" : "Written" };
      return { said: now.form === "window" || now.shown !== undefined ? "Shown again" : "Changed" };
    case "link":
      return { said: now.by === "agent" ? "Written by the agent" : "Written by you" };
    case "started_to":
      return { said: row.sub === undefined ? "Started" : `Started from ${row.sub}` };
    default:
      return { said: "Changed" };
  }
}

/**
 * The timeline of the rows the list shows, so a search that narrows the list narrows this. `acts` opens a sketch as it stood; every other point
 * opens what its row opens.
 */
export function timelineOf(session: Session, groups: readonly ArtifactGroup[], acts: TimelineActs): Timeline {
  const rows = new Map(groups.flatMap((group) => group.rows).map((row) => [row.key, row] as const));
  const timed = session.versions !== undefined;
  const history: Version[] = session.versions === undefined ? session.attachments.map((attachment) => ({ attachment })) : session.versions.map((one) => ({ at: one.at, attachment: one.attachment }));
  const first = history.flatMap((one) => (one.at === undefined ? [] : [Date.parse(one.at)])).sort((a, b) => a - b)[0] ?? 0;

  const byLine = new Map<string, { lane: LaneId; versions: Version[] }>();
  for (const one of history) {
    const who = identityOf(one.attachment);
    if (who === undefined || !rows.has(who.key)) continue;
    const held = byLine.get(who.key);
    if (held === undefined) byLine.set(who.key, { lane: who.lane, versions: [one] });
    else held.versions.push(one);
  }
  // A row the history does not name appeared with the Job.
  for (const one of session.attachments) {
    const who = identityOf(one);
    if (who !== undefined && rows.has(who.key) && !byLine.has(who.key)) byLine.set(who.key, { lane: who.lane, versions: [{ ...(timed ? { at: new Date(first).toISOString() } : {}), attachment: one }] });
  }

  let order = 0;
  const lines: TimelineLine[] = [...byLine].map(([key, { lane, versions }]) => {
    const row = rows.get(key)!;
    const points = versions.map((version, index): TimelinePoint => {
      const t = version.at === undefined ? order++ : Date.parse(version.at);
      const what = whatOf(versions[index - 1]?.attachment, version.attachment, row);
      const sketch = version.attachment.kind === "sketch" ? version.attachment : undefined;
      const open = sketch === undefined ? row.open : () => acts.sketch(sketch.id, version.at);
      return { key: `${key}@${index}`, t, said: what.said, ...(what.Glyph === undefined ? {} : { Glyph: what.Glyph }), ...(what.tone === undefined ? {} : { tone: what.tone }), ...(open === undefined ? {} : { open }) };
    });
    return { key, lane, Glyph: row.Glyph, said: row.said, title: row.title, points, parents: [] };
  });
  lines.sort((a, b) => a.points[0]!.t - b.points[0]!.t);

  const line = new Map(lines.map((one) => [one.key, one] as const));
  const all = (key: string) => byLine.get(key)!.versions.map((one) => one.attachment);
  for (const one of lines) {
    const parents: TimelineLine["parents"] = [];
    for (const attachment of all(one.key)) {
      if (attachment.kind === "pull_request") {
        for (const number of attachment.closes ?? []) if (line.has(`issue:${number}`)) parents.push({ key: `issue:${number}`, said: "Closes" });
        for (const issue of lines) if (all(issue.key).some((it) => it.kind === "issue" && it.pull_requests?.includes(attachment.number) === true)) parents.push({ key: issue.key, said: "Closes" });
      }
      if (attachment.kind === "sketch" && attachment.against !== undefined) {
        const earlier = lines.find((other) => other.key !== one.key && other.lane === "sketch" && other.title === attachment.against!.title);
        if (earlier !== undefined) parents.push({ key: earlier.key, said: "Against" });
      }
      if (attachment.kind === "started_to") parents.push({ key: "job", said: rows.get(one.key)?.sub });
    }
    const seen = new Set<string>();
    const unique = parents.filter((one) => !seen.has(one.key) && (seen.add(one.key), true));
    one.parents = unique.length === 0 ? [{ key: "job" }] : unique.map((it) => (it.said === undefined ? { key: it.key } : { key: it.key, said: it.said }));
  }

  const times = lines.flatMap((one) => one.points.map((point) => point.t));
  return { lines, start: times.length === 0 ? 0 : Math.min(...times), end: times.length === 0 ? 0 : Math.max(...times), timed };
}

// ---- Where each thing is drawn ------------------------------------------------------------------------------------------------------

export const POINT = 24;
const ROW = 52;
const ROOM = 24;
/** How far down a row its line runs; the title sits above it. */
const LINE_AT = 34;
export const LANE_LABEL = 144;
const GAP = 14;
const CHAR = 7;
/** The least a title is given before the line beside it is moved to another track. */
const LEAST_TITLE = 64;
/** Points of one line closer than this widen the plot past the panel, which then scrolls. */
const CLOSEST = 24;
/** The least between two ticks' labels. */
const TICK_ROOM = 64;
const MINUTE = 60_000;
const TICK_STEPS = [5, 10, 15, 30, 60, 120, 240].map((one) => one * MINUTE);

export type Placed = { line: TimelineLine; y: number; xs: number[]; from: number; to: number; /** The most its title may draw before it is cut with an ellipsis. */ title: number };
export type Path = { key: string; d: string; said?: string; at: { x: number; y: number } };
export type Tick = { x: number; said?: string };
export type Lane = { id: LaneId; label: string; top: number; height: number };

export type Layout = {
  /** The width drawn: the panel's, or wider where points would sit closer than they can be pressed. */
  width: number;
  /** Whether it is wider than the panel, and so scrolls. */
  scrolls: boolean;
  height: number;
  placed: Placed[];
  paths: Path[];
  ticks: Tick[];
  /** The Job's own line, in the take that draws one. */
  job?: { y: number; from: number; to: number };
  lanes: Lane[];
};

const naturalTitle = (title: string) => Math.min(title.length, 40) * CHAR + 8;

/** The first track on a side whose last line ends before this one starts, or a new one. */
function track(ends: number[], from: number): number {
  const free = ends.findIndex((end) => end + GAP <= from);
  return free < 0 ? ends.length : free;
}

/**
 * Time laid across the panel's width, so the whole Job shows. Only where two points of one line would sit closer than they can be
 * pressed is it spread wider than the panel.
 */
function scaleOf(timeline: Timeline, left: number, panel: number): { x: (t: number) => number; width: number; per: number } {
  const span = timeline.end - timeline.start;
  let closest = Infinity;
  for (const line of timeline.lines) {
    for (let at = 1; at < line.points.length; at++) {
      const gap = line.points[at]!.t - line.points[at - 1]!.t;
      if (gap > 0) closest = Math.min(closest, gap);
    }
  }
  const fit = span === 0 ? 0 : (panel - left - ROOM) / span;
  const per = Math.max(fit, closest === Infinity ? 0 : CLOSEST / closest);
  return { x: (t) => left + (t - timeline.start) * per, width: Math.max(panel, left + span * per + ROOM), per };
}

/** A tick on a round local clock time, as many as read across the width: the smallest step that leaves the labels room. */
function ticksOf(timeline: Timeline, x: (t: number) => number, per: number, clock: (ms: number) => string): Tick[] {
  if (!timeline.timed || per === 0) return [];
  const step = TICK_STEPS.find((one) => one * per >= TICK_ROOM) ?? TICK_STEPS[TICK_STEPS.length - 1]!;
  const ticks: Tick[] = [];
  const zone = new Date(timeline.start).getTimezoneOffset() * MINUTE;
  for (let t = Math.ceil((timeline.start - zone) / step) * step + zone; t <= timeline.end; t += step) ticks.push({ x: x(t), said: clock(t) });
  return ticks;
}

function placeLine(line: TimelineLine, x: (t: number) => number, y: number): Placed {
  const xs = line.points.map((point) => x(point.t));
  const from = xs[0]! - POINT / 2;
  const title = naturalTitle(line.title);
  return { line, y, xs, from, to: Math.max(xs[xs.length - 1]! + POINT / 2, from + POINT + Math.min(title, LEAST_TITLE)), title };
}

/** Each title is given the room to the next line on its track, or to the edge, and is cut with an ellipsis where that is less than it wants. */
function fitTitles(placed: readonly Placed[], edge: number): Placed[] {
  return placed.map((one) => {
    const next = placed.filter((other) => other.y === one.y && other.from > one.from).sort((a, b) => a.from - b.from)[0];
    const room = (next === undefined ? edge : next.from - GAP) - one.xs[0]! + POINT / 2;
    return { ...one, title: Math.max(POINT, Math.min(one.title, room)) };
  });
}

/** The path that branches a line off its parent: along the parent to where the line begins, then across to it. */
function pathsOf(placed: readonly Placed[], jobY: number | undefined): Path[] {
  const at = new Map(placed.map((one) => [one.line.key, one] as const));
  return placed.flatMap((child) =>
    child.line.parents.flatMap((parent): Path[] => {
      const x = child.xs[0]!;
      const label = parent.said === undefined ? {} : { said: parent.said };
      if (parent.key === "job") return jobY === undefined ? [] : [{ key: `${child.line.key}<job`, d: `M ${x} ${jobY} V ${child.y}`, ...label, at: { x, y: (jobY + child.y) / 2 } }];
      const above = at.get(parent.key);
      if (above === undefined) return [];
      const born = child.line.points[0]!.t;
      const index = above.line.points.reduce((last, point, i) => (point.t <= born ? i : last), 0);
      const from = above.xs[index]!;
      return [{ key: `${child.line.key}<${parent.key}`, d: `M ${from} ${above.y} H ${x} V ${child.y}`, ...label, at: { x, y: (above.y + child.y) / 2 } }];
    }),
  );
}

/** One Job line through the middle, artifacts branching above and below it where they appeared, the whole Job across `panel` pixels. */
export function spineOf(timeline: Timeline, panel: number, clock: (ms: number) => string): Layout {
  const left = ROOM + POINT;
  const { x, width, per } = scaleOf(timeline, left, panel);
  const ends = { up: [] as number[], down: [] as number[] };
  const sides = new Map<string, "up" | "down">();
  const set: { one: Placed; side: "up" | "down"; at: number }[] = [];
  for (const [index, line] of timeline.lines.entries()) {
    const one = placeLine(line, x, 0);
    const parent = line.parents.map((it) => sides.get(it.key)).find((it) => it !== undefined);
    const free = { up: track(ends.up, one.from), down: track(ends.down, one.from) };
    const side = parent !== undefined ? parent : free.up < free.down ? "up" : free.down < free.up ? "down" : index % 2 === 0 ? "up" : "down";
    const at = free[side];
    ends[side][at] = one.to;
    sides.set(line.key, side);
    set.push({ one, side, at });
  }
  const jobY = ends.up.length * ROW + LINE_AT;
  const placed = fitTitles(
    set.map(({ one, side, at }) => ({ ...one, y: side === "up" ? jobY - (at + 1) * ROW : jobY + (at + 1) * ROW })),
    width - ROOM,
  );
  return {
    width,
    scrolls: width > panel,
    height: (ends.up.length + 1 + ends.down.length) * ROW + 16,
    placed,
    paths: pathsOf(placed, jobY),
    ticks: ticksOf(timeline, x, per, clock),
    job: { y: jobY, from: left - POINT / 2, to: width - ROOM },
    lanes: [],
  };
}

/** A lane to each kind that has an artifact, each artifact a line in its lane, and the joins across them. The axis stands for the Job. */
export function lanesOf(timeline: Timeline, panel: number, clock: (ms: number) => string): Layout {
  const { x, width, per } = scaleOf(timeline, LANE_LABEL + ROOM + POINT / 2, panel);
  const lanes: Lane[] = [];
  const all: Placed[] = [];
  let top = 0;
  for (const lane of LANES) {
    const inside = timeline.lines.filter((line) => line.lane === lane.id);
    if (inside.length === 0) continue;
    const ends: number[] = [];
    const rows = inside.map((line) => {
      const one = placeLine(line, x, 0);
      const at = track(ends, one.from);
      ends[at] = one.to;
      return { one, at };
    });
    lanes.push({ id: lane.id, label: lane.label, top, height: ends.length * ROW });
    for (const { one, at } of rows) all.push({ ...one, y: top + at * ROW + LINE_AT });
    top += ends.length * ROW;
  }
  const placed = fitTitles(all, width - ROOM);
  return {
    width,
    scrolls: width > panel,
    height: top + 8,
    placed,
    paths: pathsOf(placed, undefined),
    ticks: ticksOf(timeline, x, per, clock),
    lanes,
  };
}

/**
 * The point an arrow reaches from `from`: along its line for left and right, and for up and down the nearest point, across, on the next row
 * that way. Rows are told apart by where their line runs, so two artifacts sharing a track are one row.
 */
export function neighbour(placed: readonly Placed[], from: { line: string; index: number }, arrow: "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown"): { line: string; index: number } | undefined {
  const here = placed.find((one) => one.line.key === from.line);
  if (here === undefined) return undefined;
  if (arrow === "ArrowLeft" || arrow === "ArrowRight") {
    const index = from.index + (arrow === "ArrowRight" ? 1 : -1);
    return index < 0 || index >= here.xs.length ? undefined : { line: from.line, index };
  }
  const x = here.xs[from.index]!;
  const toward = arrow === "ArrowDown" ? (y: number) => y > here.y : (y: number) => y < here.y;
  const rows = [...new Set(placed.map((one) => one.y).filter(toward))].sort((a, b) => (arrow === "ArrowDown" ? a - b : b - a));
  for (const y of rows) {
    let best: { line: string; index: number; away: number } | undefined;
    for (const one of placed) {
      if (one.y !== y) continue;
      for (const [index, at] of one.xs.entries()) {
        const away = Math.abs(at - x);
        if (best === undefined || away < best.away) best = { line: one.line.key, index, away };
      }
    }
    if (best !== undefined) return { line: best.line, index: best.index };
  }
  return undefined;
}
