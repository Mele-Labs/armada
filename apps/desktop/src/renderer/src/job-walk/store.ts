// Where each walk stands, outside the tab that draws it: the take on screen, the step, and whether he has seen the tab or approved the walk. Keyed by Job and walk, so a walk left
// for another Job and come back to is as he left it, and so is one whose tab was dragged to another pane.

import { useSyncExternalStore } from "react";

export type WalkState = {
  take: number;
  /** The step on screen, from 0. */
  step: number;
  /** The tab has been in front of him since it arrived: it stops pulsing. */
  seen: boolean;
  approved: boolean;
};

const FRESH: WalkState = {
  take: 0,
  step: 0,
  seen: false,
  approved: false,
};

const key = (jobId: string, walk: string) => `${jobId}\n${walk}`;
let states = new Map<string, WalkState>();
/** The walks whose tab has been opened once, so one he closes by its × is not opened again. */
let arrived = new Set<string>();
let version = 0;
const listeners = new Set<() => void>();

const change = (jobId: string, walk: string, next: Partial<WalkState>): void => {
  const at = key(jobId, walk);
  states = new Map(states).set(at, {
    ...(states.get(at) ?? FRESH),
    ...next,
  });
  version += 1;
  listeners.forEach((hear) => hear());
};

const subscribe = (hear: () => void) => {
  listeners.add(hear);
  return () => listeners.delete(hear);
};

/** Every walk as it was before anything happened: a mock starts from here. */
export function forgetWalks(): void {
  states = new Map();
  arrived = new Set();
  version += 1;
  listeners.forEach((hear) => hear());
}

export const stateOf = (jobId: string, walk: string): WalkState =>
  states.get(key(jobId, walk)) ?? FRESH;
export const isApproved = (jobId: string, walk: string): boolean => stateOf(jobId, walk).approved;

export function useWalkState(jobId: string, walk: string | undefined): WalkState {
  return useSyncExternalStore(subscribe, () =>
    walk === undefined ? FRESH : (states.get(key(jobId, walk)) ?? FRESH),
  );
}

/** Changes whenever any walk does, for what is read from them across Jobs. */
export const useWalksVersion = (): number => useSyncExternalStore(subscribe, () => version);

export const pickTake = (jobId: string, walk: string, take: number): void =>
  change(jobId, walk, {
    take,
  });
export const goToStep = (jobId: string, walk: string, step: number): void =>
  change(jobId, walk, {
    step,
  });
export const markSeen = (jobId: string, walk: string): void => {
  if (!stateOf(jobId, walk).seen)
    change(jobId, walk, {
      seen: true,
    });
};
export const approveWalk = (jobId: string, walk: string): void =>
  change(jobId, walk, {
    approved: true,
  });

/** Whether the walk's tab has been opened once; the first call says no and remembers. */
export function arrivedBefore(jobId: string, walk: string): boolean {
  const at = key(jobId, walk);
  if (arrived.has(at)) return true;
  arrived.add(at);
  return false;
}
