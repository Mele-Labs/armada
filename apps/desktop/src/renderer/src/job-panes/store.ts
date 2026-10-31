// What each open Job's area looks like, and the named layouts it can take. Held in memory for now; keeping them is the next piece. A Job's arrangement is its own: rearranging
// one never moves another. A Job that has not been arranged or given a layout follows the layout marked for all Jobs.

import { useSyncExternalStore } from "react";

import { arrive, build, copyOf, type Arrangement, type TabKind } from "./tree";

export const DEFAULT_LAYOUT = "Default";

/** Today's arrangement: the Job's thread, and Done, Now and Open to its right. */
const defaultArrangement = (): Arrangement => build({ dir: "row", ratio: 0.62, a: ["thread"], b: ["done", "now", "open"] }, "now");

/** **The mock's**: two saved layouts to pick from until layouts are kept for real. */
const seeded = (): Record<string, Arrangement> => ({
  "QA split": build({ dir: "row", ratio: 0.55, a: { dir: "column", ratio: 0.6, a: ["thread"], b: ["terminal"] }, b: ["browser"] }),
  Review: build({ dir: "row", ratio: 0.5, a: ["thread"], b: ["diff"] }),
});

type Entry = {
  arrangement: Arrangement;
  /** The layout it was taken from. */
  layout: string;
  /** Rearranged since, and not yet updated, saved or reset. */
  edited: boolean;
  /** Arranged or given a layout by the person, so a change of the layout for all Jobs leaves it be. */
  chosen: boolean;
};

type State = {
  jobs: Readonly<Record<string, Entry>>;
  saved: Readonly<Record<string, Arrangement>>;
  /** The layout every Job takes that has not been given another. */
  forAll: string;
};

let state: State = { jobs: {}, saved: seeded(), forAll: DEFAULT_LAYOUT };
const listeners = new Set<() => void>();

const told = () => listeners.forEach((hear) => hear());

function set(next: State): void {
  state = next;
  told();
}

const layoutNamed = (name: string, saved: State["saved"]): Arrangement => {
  const found = name === DEFAULT_LAYOUT ? undefined : saved[name];
  return copyOf(found ?? defaultArrangement());
};

/** The Job's entry, made on first sight from the layout for all Jobs, and brought to it while the person has not chosen otherwise. No notice: the same answer every time. */
function ensure(jobId: string): Entry {
  const held = state.jobs[jobId];
  if (held !== undefined && (held.chosen || held.layout === state.forAll)) return held;
  const made: Entry = { arrangement: layoutNamed(state.forAll, state.saved), layout: state.forAll, edited: false, chosen: false };
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

/** A change of arrangement the person made: this Job is edited. */
export function rearrange(jobId: string, change: (arrangement: Arrangement) => Arrangement): void {
  put(jobId, (entry) => {
    const next = change(entry.arrangement);
    return next === entry.arrangement ? entry : { ...entry, arrangement: next, edited: true, chosen: true };
  });
}

/** Which tab is in front and which pane has the focus: where the person is, not how the area is laid out. */
export function look(jobId: string, change: (arrangement: Arrangement) => Arrangement): void {
  put(jobId, (entry) => {
    const next = change(entry.arrangement);
    return next === entry.arrangement ? entry : { ...entry, arrangement: next };
  });
}

/** A tab the Job opened itself. */
export function jobOpens(jobId: string, kind: TabKind): void {
  rearrange(jobId, (arrangement) => arrive(arrangement, kind));
}

export function takeLayout(jobId: string, name: string): void {
  put(jobId, () => ({ arrangement: layoutNamed(name, state.saved), layout: name, edited: false, chosen: true }));
}

/** The Job's arrangement becomes what the layout is. Default stays as it is. */
export function updateLayout(jobId: string): void {
  const entry = ensure(jobId);
  if (entry.layout === DEFAULT_LAYOUT) return;
  set({ ...state, saved: { ...state.saved, [entry.layout]: copyOf(entry.arrangement) }, jobs: { ...state.jobs, [jobId]: { ...entry, edited: false } } });
}

/** A name is free when no layout has it. */
export const nameTaken = (name: string): boolean => name.trim() === DEFAULT_LAYOUT || state.saved[name.trim()] !== undefined;

export function saveLayoutAs(jobId: string, name: string): void {
  const entry = ensure(jobId);
  const label = name.trim();
  if (label === "" || label === DEFAULT_LAYOUT) return;
  set({ ...state, saved: { ...state.saved, [label]: copyOf(entry.arrangement) }, jobs: { ...state.jobs, [jobId]: { ...entry, layout: label, edited: false, chosen: true } } });
}

export function resetLayout(jobId: string): void {
  const entry = ensure(jobId);
  put(jobId, () => ({ ...entry, arrangement: layoutNamed(entry.layout, state.saved), edited: false }));
}

/** The Job's layout is the one every Job takes from now on, and so is every Job that has not been given another. */
export function setLayoutForAll(jobId: string): void {
  const entry = ensure(jobId);
  set({ ...state, forAll: entry.layout });
}
