// The Walk tool's state, beside the review store and not in it: what was found about where a pull request runs, the
// places Armada remembers, and how far a walk has been played. **Built once per review store** and read through
// `useWalks`, so a window that leaves the Reviews tab and comes back finds it as it was. The tool's tile, its kept mark
// and its out-of-date mark are the review store's own run for the tool (`runTool`), started once the place is known.

import { useSyncExternalStore } from "react";

import type { ReviewPr, ToolDef } from "./model";
import { useReviewStore } from "./store";
import type { ReviewStore } from "./store";
import { areaOf, fromPullRequest, placeSaid, rememberedFor, waitsOn } from "./walk-model";
import type { Phase, Place, Remembered, Tried, WalkSeed, WalkState } from "./walk-model";

/** How long one source takes to read, and one setup line to run, in the mock. */
export const LOOK_MS = 320;

/** The sheet that is up: asking where this pull request runs, or the places Armada remembers. */
export type WalkSheet = { kind: "ask"; key: string } | { kind: "places" } | undefined;

export type WalkSnapshot = {
  memory: readonly Remembered[];
  by: Readonly<Record<string, WalkState>>;
  sheet: WalkSheet;
};

export type WalkStore = {
  get: () => WalkSnapshot;
  subscribe: (listener: () => void) => () => void;
  /** The tile was pressed: shows the walk kept for this pull request, or asks before it spends. */
  press: (pr: ReviewPr, tool: ToolDef) => void;
  /** Starts again from the first source, for this pull request. */
  begin: (pr: ReviewPr, tool: ToolDef) => void;
  /** His answer to the question, remembered for `area` where one is given. */
  answer: (pr: ReviewPr, tool: ToolDef, place: Place, remember: string | undefined) => void;
  /** Esc on the sheet leaves the question open on the page. */
  closeSheet: () => void;
  ask: (key: string) => void;
  showPlaces: () => void;
  editPlace: (id: string, place: Place) => void;
  forgetPlace: (id: string) => void;
  /** Sends one request of an API walk. */
  send: (key: string, id: string) => void;
  /** Puts the walk window up on its first step, or moves it by one, or takes it down. */
  walkIt: (key: string) => void;
  moveWindow: (key: string, by: 1 | -1) => void;
  closeWindow: (key: string) => void;
  plan: (key: string) => WalkSeed["plans"][string] | undefined;
};

const EMPTY: WalkSeed = { plans: {}, memory: [] };

let ids = 0;

export function createWalkStore(reviews: ReviewStore, seed: WalkSeed): WalkStore {
  const listeners = new Set<() => void>();
  let snapshot: WalkSnapshot = { memory: seed.memory, by: {}, sheet: undefined };
  const set = (next: Partial<WalkSnapshot>) => {
    snapshot = { ...snapshot, ...next };
    listeners.forEach((one) => one());
  };
  const change = (key: string, by: Partial<WalkState>) => {
    const was = snapshot.by[key];
    if (was !== undefined) set({ by: { ...snapshot.by, [key]: { ...was, ...by } } });
  };
  const after = (run: () => void, ms = LOOK_MS) => void window.setTimeout(run, ms);
  const tell = (key: string, line: Tried) => {
    const was = snapshot.by[key];
    if (was === undefined) return;
    const at = was.tried.findIndex((one) => one.source === line.source);
    change(key, { tried: at < 0 ? [...was.tried, line] : was.tried.map((one, n) => (n === at ? line : one)) });
  };

  const write = (pr: ReviewPr, tool: ToolDef, again: boolean) => {
    if (again) reviews.rerun(pr.key, tool);
    else reviews.runTool(pr.key, tool);
  };

  // The place is known. A pulled-down change is checked out and run before anything is written, a line at a time.
  const settle = (pr: ReviewPr, tool: ToolDef, place: Place, again: boolean) => {
    change(pr.key, { place });
    if (place.how === "deployed") return write(pr, tool, again);
    const slot = seed.plans[pr.key]?.slot ?? 1;
    const lines: { id: string; said: string; code?: string }[] = [
      { id: "checkout", said: `Check out ${pr.branch} into slot ${slot}` },
      { id: "run", said: "Run", code: place.command },
      { id: "wait", said: "Wait for", code: waitsOn(place) },
    ];
    change(pr.key, { setup: lines.map((one) => ({ ...one, phase: "pending" as Phase })) });
    const move = (at: number) => {
      if (at >= lines.length) return write(pr, tool, again);
      const phase = (id: string, to: Phase) => {
        const was = snapshot.by[pr.key]?.setup ?? [];
        change(pr.key, { setup: was.map((one) => (one.id === id ? { ...one, phase: to } : one)) });
      };
      phase(lines[at]!.id, "running");
      after(() => {
        phase(lines[at]!.id, "done");
        move(at + 1);
      });
    };
    move(0);
  };

  const store: WalkStore = {
    get: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    plan: (key) => seed.plans[key],
    press: (pr, tool) => {
      const had = snapshot.by[pr.key];
      if (had === undefined) return reviews.spend(tool.name, () => store.begin(pr, tool));
      reviews.setView(pr.key, tool.id);
      if (had.tried.at(-1)?.state === "asking") set({ sheet: { kind: "ask", key: pr.key } });
    },
    begin: (pr, tool) => {
      const again = snapshot.by[pr.key] !== undefined && reviews.get().work[pr.key]?.runs[tool.id] !== undefined;
      set({ by: { ...snapshot.by, [pr.key]: { head: pr.head, tried: [{ source: "pr", state: "looking", said: "" }], place: undefined, setup: [], sent: {}, window: undefined } } });
      reviews.setView(pr.key, tool.id);
      after(() => {
        const found = fromPullRequest(pr);
        if (found !== undefined) {
          tell(pr.key, { source: "pr", state: "found", said: found.said, by: found.by });
          return settle(pr, tool, { how: "deployed", address: found.address, command: "", api: "" }, again);
        }
        tell(pr.key, { source: "pr", state: "nothing", said: "No preview link in the comments or the checks" });
        tell(pr.key, { source: "memory", state: "looking", said: "" });
        after(() => {
          const kept = rememberedFor(snapshot.memory, pr);
          if (kept !== undefined) {
            tell(pr.key, { source: "memory", state: "found", said: placeSaid(kept.place), by: kept.area });
            return settle(pr, tool, kept.place, again);
          }
          tell(pr.key, { source: "memory", state: "nothing", said: `Nothing remembered for ${areaOf(pr)}/ in ${pr.repository}` });
          tell(pr.key, { source: "ask", state: "asking", said: "" });
          set({ sheet: { kind: "ask", key: pr.key } });
        });
      });
    },
    answer: (pr, tool, place, remember) => {
      const again = reviews.get().work[pr.key]?.runs[tool.id] !== undefined;
      tell(pr.key, { source: "ask", state: "found", said: placeSaid(place) });
      const area = remember?.trim().replace(/\/$/, "");
      set({
        sheet: undefined,
        ...(area === undefined || area === ""
          ? {}
          : { memory: [...snapshot.memory.filter((one) => !(one.repository === pr.repository && one.area === area)), { id: `place${(ids += 1)}`, repository: pr.repository, area, place }] }),
      });
      settle(pr, tool, place, again);
    },
    closeSheet: () => set({ sheet: undefined }),
    ask: (key) => set({ sheet: { kind: "ask", key } }),
    showPlaces: () => set({ sheet: { kind: "places" } }),
    editPlace: (id, place) => set({ memory: snapshot.memory.map((one) => (one.id === id ? { ...one, place } : one)) }),
    forgetPlace: (id) => set({ memory: snapshot.memory.filter((one) => one.id !== id) }),
    send: (key, id) => {
      const was = snapshot.by[key];
      if (was === undefined || was.sent[id] === "running") return;
      change(key, { sent: { ...was.sent, [id]: "running" } });
      after(() => change(key, { sent: { ...(snapshot.by[key]?.sent ?? {}), [id]: "done" } }), LOOK_MS * 2);
    },
    walkIt: (key) => change(key, { window: 0 }),
    moveWindow: (key, by) => {
      const at = snapshot.by[key]?.window;
      const last = (seed.plans[key]?.steps.length ?? 1) - 1;
      if (at !== undefined) change(key, { window: Math.min(last, Math.max(0, at + by)) });
    },
    closeWindow: (key) => change(key, { window: undefined }),
  };
  return store;
}

// ---- The window's hand-down -------------------------------------------------------------------------------

const held = new WeakMap<ReviewStore, WalkStore>();

/** Gives a review store its walks. A review store with none has an empty set: every pull request asks. */
export function provideWalks(reviews: ReviewStore, seed: WalkSeed): void {
  held.set(reviews, createWalkStore(reviews, seed));
}

export function walksOf(reviews: ReviewStore): WalkStore {
  const one = held.get(reviews) ?? createWalkStore(reviews, EMPTY);
  held.set(reviews, one);
  return one;
}

export const useWalks = (): WalkStore => walksOf(useReviewStore()!);

/** The walk state as it stands, live. */
export function useWalkState(): WalkSnapshot {
  const walks = useWalks();
  return useSyncExternalStore(walks.subscribe, walks.get);
}
