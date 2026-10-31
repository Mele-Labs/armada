// What each open Job's area looks like, and the named layouts it can take. Held in memory for now; keeping them is the next piece. A Job's arrangement is its own: rearranging
// one never moves another. A Job that has not been arranged or given a layout follows the layout marked for all Jobs.

import { useSyncExternalStore } from "react";

import { arrive, build, copyOf, hasKind, moveTabToEdge, moveTabToGap, moveTabToPane, shortfall, type Arrangement, type Side, type TabKind } from "./tree";

export const DEFAULT_LAYOUT = "Default";

/** Today's arrangement: the Job's thread, and Done, Now and Open stacked to its right, all in view. */
const defaultArrangement = (): Arrangement => build({ dir: "row", ratio: 0.62, a: ["thread"], b: { dir: "column", ratio: 0.18, a: ["done"], b: { dir: "column", ratio: 0.65, a: ["now"], b: ["open"] } } });

/** What the mock's browser and terminal tabs read until the tab says its own. */
const SEEDED_LABELS = { browser: "localhost:5173", terminal: "zsh" } as const;

/** **The mock's**: two saved layouts to pick from until layouts are kept for real. */
const seeded = (): Record<string, Arrangement> => ({
  "QA split": build({ dir: "row", ratio: 0.55, a: { dir: "column", ratio: 0.6, a: ["thread"], b: ["terminal"] }, b: ["browser"] }, undefined, SEEDED_LABELS),
  Review: build({ dir: "row", ratio: 0.5, a: ["thread"], b: ["changes"] }),
});

type Entry = {
  arrangement: Arrangement;
  /** The layout it was taken from. */
  layout: string;
  /** Rearranged since, and not yet updated, saved or reset. */
  edited: boolean;
  /** Arranged or given a layout by the person, so a change of the layout for all Jobs leaves it be. */
  chosen: boolean;
  /** The layout's version it was taken at: a Job not edited follows the layout when it is updated. */
  version: number;
};

type State = {
  jobs: Readonly<Record<string, Entry>>;
  saved: Readonly<Record<string, Arrangement>>;
  /** The layout every Job takes that has not been given another. */
  forAll: string;
  /** How many times each layout has been updated. */
  versions: Readonly<Record<string, number>>;
  /** Default after it was updated; absent, Default is today's arrangement. */
  base: Arrangement | undefined;
};

let state: State = { jobs: {}, saved: seeded(), forAll: DEFAULT_LAYOUT, versions: {}, base: undefined };
const listeners = new Set<() => void>();

const told = () => listeners.forEach((hear) => hear());

function set(next: State): void {
  state = next;
  told();
}

const versionOf = (name: string): number => state.versions[name] ?? 0;

const layoutNamed = (name: string): Arrangement => copyOf((name === DEFAULT_LAYOUT ? state.base : state.saved[name]) ?? defaultArrangement());

/** The Job's entry, made on first sight from the layout for all Jobs. It is brought to that layout while the person has not chosen otherwise, and to a layout's update while the Job is not edited. No notice: the same answer every time. */
function ensure(jobId: string): Entry {
  const held = state.jobs[jobId];
  if (held !== undefined) {
    const following = !held.chosen && held.layout !== state.forAll;
    const stale = !held.edited && held.version !== versionOf(held.layout);
    if (!following && !stale) return held;
  }
  const layout = held?.chosen === true ? held.layout : state.forAll;
  const made: Entry = { arrangement: layoutNamed(layout), layout, edited: false, chosen: held?.chosen ?? false, version: versionOf(layout) };
  state = { ...state, jobs: { ...state.jobs, [jobId]: made } };
  return made;
}

export type JobPanes = {
  arrangement: Arrangement;
  layout: string;
  edited: boolean;
  /** Every layout there is, Default first. */
  layouts: readonly string[];
  forAll: string;
};

export function useJobPanes(jobId: string): JobPanes {
  const seen = useSyncExternalStore(
    (hear) => {
      listeners.add(hear);
      return () => listeners.delete(hear);
    },
    () => (ensure(jobId), state),
  );
  const entry = seen.jobs[jobId]!;
  return { arrangement: entry.arrangement, layout: entry.layout, edited: entry.edited, layouts: [DEFAULT_LAYOUT, ...Object.keys(seen.saved)], forAll: seen.forAll };
}

function put(jobId: string, change: (entry: Entry) => Entry): void {
  const entry = ensure(jobId);
  set({ ...state, jobs: { ...state.jobs, [jobId]: change(entry) } });
}

/** The Job's arrangement as it stands now. */
export const peek = (jobId: string): Arrangement => ensure(jobId).arrangement;

// ---- Where the person has been, for ⌘Z --------------------------------------------------------------------

type History = { past: Entry[]; future: Entry[]; merge?: string; at: number };
const histories = new Map<string, History>();
const HISTORY = 50;
/** A run of one kind of change, a drag along a line, is one step back. */
const MERGE_MS = 600;

function remember(jobId: string, was: Entry, merge?: string): void {
  const history = histories.get(jobId) ?? { past: [], future: [], at: 0 };
  const now = Date.now();
  const joins = merge !== undefined && history.merge === merge && now - history.at < MERGE_MS;
  history.at = now;
  if (merge !== undefined) history.merge = merge;
  else delete history.merge;
  if (!joins) history.past = [...history.past.slice(-(HISTORY - 1)), was];
  history.future = [];
  histories.set(jobId, history);
}

/** The area's size in px as drawn, which the panes' minimums are held against. */
const areas = new Map<string, { w: number; h: number }>();
export function measured(jobId: string, size: { w: number; h: number }): void {
  areas.set(jobId, size);
}

/** Whether a change leaves every pane its minimum, or in an area too small for that, no pane further short than before. */
function allowed(jobId: string, was: Arrangement, next: Arrangement): boolean {
  const size = areas.get(jobId);
  return size === undefined || size.w === 0 || shortfall(next, size) <= shortfall(was, size);
}

/** A change of arrangement the person made: this Job is edited. One that would squeeze a pane below its minimum is refused, unless it is forced, as a Job opening a tab is. `merge` names a run of changes that is one step for ⌘Z. */
export function rearrange(jobId: string, change: (arrangement: Arrangement) => Arrangement, merge?: string, force = false): void {
  const entry = ensure(jobId);
  const next = change(entry.arrangement);
  if (next === entry.arrangement || (!force && !allowed(jobId, entry.arrangement, next))) return;
  remember(jobId, entry, merge);
  set({ ...state, jobs: { ...state.jobs, [jobId]: { ...entry, arrangement: next, edited: true, chosen: true } } });
}

/** Where a tab dragged over a pane would land, and whether the pane can take it. */
export function droppable(jobId: string, tabId: string, paneId: string, zone: Side | "middle"): boolean {
  const was = ensure(jobId).arrangement;
  const next = zone === "middle" ? moveTabToPane(was, tabId, paneId) : moveTabToEdge(was, tabId, paneId, zone);
  return next !== was && allowed(jobId, was, next);
}

/** Whether a tab dragged over the gap of a split can be dropped there: a new pane between the two sides, with room for every pane. */
export function droppableGap(jobId: string, tabId: string, splitId: string): boolean {
  const was = ensure(jobId).arrangement;
  const next = moveTabToGap(was, tabId, splitId);
  return next !== was && allowed(jobId, was, next);
}

function go(jobId: string, from: "past" | "future"): void {
  const history = histories.get(jobId);
  const target = history?.[from].pop();
  if (history === undefined || target === undefined) return;
  const entry = ensure(jobId);
  (from === "past" ? history.future : history.past).push(entry);
  delete history.merge;
  set({ ...state, jobs: { ...state.jobs, [jobId]: target } });
}

export const undo = (jobId: string): void => go(jobId, "past");
export const redo = (jobId: string): void => go(jobId, "future");

/** Which tab is in front and which pane has the focus: where the person is, not how the area is laid out. */
export function look(jobId: string, change: (arrangement: Arrangement) => Arrangement): void {
  put(jobId, (entry) => {
    const next = change(entry.arrangement);
    return next === entry.arrangement ? entry : { ...entry, arrangement: next };
  });
}

/** A tab the Job opened itself. One that joins a pane is content and leaves the layout as it was; one that makes a split is arrangement. */
export function jobOpens(jobId: string, kind: TabKind, label?: string, page?: string): void {
  if (hasKind(peek(jobId), kind)) look(jobId, (arrangement) => arrive(arrangement, kind, label, page));
  else rearrange(jobId, (arrangement) => arrive(arrangement, kind, label, page), undefined, true);
}

export function takeLayout(jobId: string, name: string): void {
  remember(jobId, ensure(jobId));
  put(jobId, () => ({ arrangement: layoutNamed(name), layout: name, edited: false, chosen: true, version: versionOf(name) }));
}

/** The Job's arrangement becomes what the layout is, for every Job on it that is not edited. */
export function updateLayout(jobId: string): void {
  const entry = ensure(jobId);
  if (!entry.edited) return;
  const version = versionOf(entry.layout) + 1;
  const kept = copyOf(entry.arrangement);
  set({
    ...state,
    versions: { ...state.versions, [entry.layout]: version },
    ...(entry.layout === DEFAULT_LAYOUT ? { base: kept } : { saved: { ...state.saved, [entry.layout]: kept } }),
    jobs: { ...state.jobs, [jobId]: { ...entry, edited: false, version } },
  });
}

/** A name is free when no layout has it. */
export const nameTaken = (name: string): boolean => name.trim() === DEFAULT_LAYOUT || state.saved[name.trim()] !== undefined;

export function saveLayoutAs(jobId: string, name: string): void {
  const entry = ensure(jobId);
  const label = name.trim();
  if (label === "" || label === DEFAULT_LAYOUT) return;
  set({ ...state, saved: { ...state.saved, [label]: copyOf(entry.arrangement) }, jobs: { ...state.jobs, [jobId]: { ...entry, layout: label, edited: false, chosen: true, version: 0 } } });
}

export function resetLayout(jobId: string): void {
  const entry = ensure(jobId);
  remember(jobId, entry);
  put(jobId, () => ({ ...entry, arrangement: layoutNamed(entry.layout), edited: false, version: versionOf(entry.layout) }));
}

/** The named layout is the one every Job takes from now on, and so is every Job that has not been given another. */
export function setLayoutForAll(layout: string): void {
  set({ ...state, forAll: layout });
}
