// Where each walk stands, outside the tab that draws it: the take on screen, the step, and whether he has seen the tab or approved the walk. Keyed by Job and walk, so a walk left
// for another Job and come back to is as he left it, and so is one whose tab was dragged to another pane.

import { useSyncExternalStore } from "react";
import type { PanelHeldNote } from "@armada/studios/api";

export type WalkState = {
  take: number;
  /** The step on screen, from 0. */
  step: number;
  /** The tab has been in front of him since it arrived: it stops pulsing. */
  seen: boolean;
  approved: boolean;
  /** What the Job says of the page: still changing after it took the notes, ready once it shows the page again. Absent until it has said. */
  readiness?: "ready" | "working";
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
/** The notes taken on each Job's walks and not sent. They belong to the Job: leaving it, closing a tab or approving without them leaves them held. */
let held = new Map<string, readonly PanelHeldNote[]>();
/** Walks he pressed open, waiting for `sync.tsx` to bring their tab to the front. */
let pressed: { jobId: string; walk: string }[] = [];
let minted = 0;
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
  held = new Map();
  pressed = [];
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

/** A walk he pressed open, from Needs you or the Job's own rows: its tab comes to the front, and one he had closed or never had is made. */
export function bringWalkForward(jobId: string, walk: string): void {
  pressed = [...pressed, { jobId, walk }];
  version += 1;
  listeners.forEach((hear) => hear());
}

/** The walks pressed open and not yet brought forward. */
export const pressedWalks = (): readonly { jobId: string; walk: string }[] => pressed;

/** A pressed walk brought forward, or one that has no Job to bring it to. */
export function settlePressed(jobId: string, walk: string): void {
  pressed = pressed.filter((one) => one.jobId !== jobId || one.walk !== walk);
}

/** Whether the walk's tab has been opened once; the first call says no and remembers. */
export function arrivedBefore(jobId: string, walk: string): boolean {
  const at = key(jobId, walk);
  if (arrived.has(at)) return true;
  arrived.add(at);
  return false;
}

// ---- Notes held on a Job ---------------------------------------------------------------------------------

const NONE: readonly PanelHeldNote[] = [];

const changeHeld = (jobId: string, notes: readonly PanelHeldNote[]): void => {
  held = new Map(held);
  if (notes.length === 0) held.delete(jobId);
  else held.set(jobId, notes);
  version += 1;
  listeners.forEach((hear) => hear());
};

/** The notes held on the Job, live. */
export const useHeldNotes = (jobId: string | undefined): readonly PanelHeldNote[] =>
  useSyncExternalStore(subscribe, () => (jobId === undefined ? NONE : (held.get(jobId) ?? NONE)));

/** A note taken on `on`, which names the walk and the take it was taken on. */
export const holdNote = (jobId: string, said: string, on: string): void =>
  changeHeld(jobId, [
    ...(held.get(jobId) ?? NONE),
    {
      id: `held-${(minted += 1)}`,
      at: new Date().toISOString(),
      said,
      on,
    },
  ]);

export function editNote(jobId: string, id: string, said: string): boolean {
  const notes = held.get(jobId) ?? NONE;
  if (said.trim() === "" || !notes.some((one) => one.id === id)) return false;
  changeHeld(
    jobId,
    notes.map((one) =>
      one.id === id
        ? {
            ...one,
            said: said.trim(),
          }
        : one,
    ),
  );
  return true;
}

export function removeNote(jobId: string, id: string): boolean {
  const notes = held.get(jobId) ?? NONE;
  if (!notes.some((one) => one.id === id)) return false;
  changeHeld(
    jobId,
    notes.filter((one) => one.id !== id),
  );
  return true;
}

/** The notes the Job holds, taken off it: they are being sent. */
export function takeHeld(jobId: string): readonly PanelHeldNote[] {
  const notes = held.get(jobId) ?? NONE;
  changeHeld(jobId, NONE);
  return notes;
}

/** The Job took the notes and is changing its page: each walk it holds reads still changing. */
export function workingOn(jobId: string, walks: readonly string[]): void {
  for (const walk of walks)
    change(jobId, walk, {
      readiness: "working",
    });
}

/** The Jobs with a page they are changing. */
export const workingJobs = (): string[] => [
  ...new Set(
    [
      ...states,
    ]
      .filter(([, one]) => one.readiness === "working")
      .map(([at]) => at.split("\n")[0]!),
  ),
];

/** The Job shows its page again: every walk it was changing is ready. */
export function showAgain(jobId: string): void {
  for (const [at, one] of states)
    if (at.startsWith(`${jobId}\n`) && one.readiness === "working")
      change(jobId, at.slice(jobId.length + 1), {
        readiness: "ready",
      });
}
