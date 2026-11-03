// What each open owner's area looks like, and the named layouts it can take. An owner is a Job, or a pull request on the Reviews page, and each kind of owner is a
// family with layouts of its own: a pull request's layout never moves a Job's. An owner's arrangement is its own: rearranging one never moves another. One that has not been
// arranged or given a layout follows the layout new ones of its family open on. **Kept across restarts on this machine** through the kept values every text box uses
// (`keep.ts`): a family's saved layouts and the one new owners open on under one key, and each owner's own arrangement under a key of its own. What ⌘Z can take back is this
// window's and is not kept.

import { createContext, useContext, useSyncExternalStore } from "react";

import { forgetKept, readKept, writeKept } from "@armada/components";

import {
  JOB_TAB_KINDS,
  PR_TAB_KINDS,
  activate,
  addTab,
  arrive,
  arriveTerminalBehind,
  build,
  copyOf,
  focusPane,
  hasKind,
  holderOf,
  showPage,
  moveTabToEdge,
  moveTabToEnd,
  moveTabToGap,
  moveTabToPane,
  panesOf,
  shortfall,
  type Arrangement,
  type Node,
  type Side,
  type TabKind,
} from "./tree";

export const DEFAULT_LAYOUT = "Default";

/** Today's arrangement: the Job's thread, and to its right Open, Done and Deferred as tabs of one pane over Now. */
const defaultArrangement = (): Arrangement =>
  build({
    dir: "row",
    ratio: 0.62,
    a: [
      "thread",
    ],
    b: {
      dir: "column",
      ratio: 0.4,
      a: [
        "open",
        "done",
        "deferred",
      ],
      b: [
        "now",
      ],
    },
  });

/** A pull request's: Overview, Files, Checks, Commits and Review as tabs of the main pane, and to its right the tools over Comments. */
const prArrangement = (): Arrangement =>
  build({
    dir: "row",
    ratio: 0.62,
    a: [
      "pr-overview",
      "pr-files",
      "pr-checks",
      "pr-commits",
      "pr-review",
    ],
    b: {
      dir: "column",
      ratio: 0.5,
      a: [],
      b: [
        "pr-comments",
      ],
    },
  });

type Entry = {
  arrangement: Arrangement;
  /** The layout it was taken from. */
  layout: string;
  /** Rearranged since, and not yet updated, saved or reset. */
  edited: boolean;
  /** Arranged or given a layout by the person, so a change of the layout for all leaves it be. */
  chosen: boolean;
  /** The layout's version it was taken at: an owner not edited follows the layout when it is updated. */
  version: number;
};

type State = {
  jobs: Readonly<Record<string, Entry>>;
  saved: Readonly<Record<string, Arrangement>>;
  /** The layout every owner takes that has not been given another. */
  forAll: string;
  /** How many times each layout has been updated. */
  versions: Readonly<Record<string, number>>;
  /** Default after it was updated; absent, Default is the family's own arrangement. */
  base: Arrangement | undefined;
};

type Shared = Pick<State, "saved" | "forAll" | "versions" | "base">;

// ---- Kept ------------------------------------------------------------------------------------------------

const SHARED = "job-panes:shared";
const JOB = "job-panes:job:";
const DEFERRED_GIVEN = "job-panes:deferred-given";

const isNode = (one: unknown, kinds: readonly string[]): boolean => {
  const node = one as Node | undefined;
  if (node === undefined || node === null || typeof node !== "object") return false;
  if (node.type === "pane")
    return (
      typeof node.id === "string" &&
      Array.isArray(node.tabs) &&
      node.tabs.every((tab) => typeof tab?.id === "string" && kinds.includes(tab.kind))
    );
  return (
    node.type === "split" &&
    typeof node.id === "string" &&
    (node.dir === "row" || node.dir === "column") &&
    typeof node.ratio === "number" &&
    isNode(node.a, kinds) &&
    isNode(node.b, kinds)
  );
};
/** A kept arrangement that is not one any more, from another version or with a kind its family has not, is not used. */
const isArrangement = (one: unknown, kinds: readonly string[]): one is Arrangement => {
  const arrangement = one as Arrangement | undefined;
  return (
    arrangement !== undefined &&
    arrangement !== null &&
    typeof arrangement === "object" &&
    typeof arrangement.focus === "string" &&
    typeof arrangement.next === "number" &&
    isNode(arrangement.root, kinds) &&
    panesOf(arrangement.root).some((pane) => pane.id === arrangement.focus)
  );
};

const reviverOf =
  (kinds: readonly string[]) =>
  (stored: unknown): Entry => {
    const one = stored as Entry;
    if (!isArrangement(one?.arrangement, kinds) || typeof one.layout !== "string")
      throw new Error("not an owner's panes");
    return {
      arrangement: one.arrangement,
      layout: one.layout,
      edited: one.edited === true,
      chosen: one.chosen === true,
      version: typeof one.version === "number" ? one.version : 0,
    };
  };

const sharedReviverOf =
  (kinds: readonly string[]) =>
  (stored: unknown): Shared => {
    const one = stored as Partial<Shared> | undefined;
    const saved = Object.fromEntries(
      Object.entries(one?.saved ?? {}).filter(([, arrangement]) =>
        isArrangement(arrangement, kinds),
      ),
    );
    return {
      saved,
      forAll: typeof one?.forAll === "string" ? one.forAll : DEFAULT_LAYOUT,
      versions: one?.versions ?? {},
      base: isArrangement(one?.base, kinds) ? one.base : undefined,
    };
  };

const reviveEntry = reviverOf(JOB_TAB_KINDS);
const reviveShared = sharedReviverOf(JOB_TAB_KINDS);

/** The arrangement with a Deferred tab at the end of the pane that holds Open, its front tab and focus as they were. */
const withDeferred = (arrangement: Arrangement): Arrangement => {
  const open = holderOf(arrangement, "open");
  if (open === undefined || hasKind(arrangement, "deferred")) return arrangement;
  const added = addTab(arrangement, open.id, "deferred");
  const fronted = open.active === undefined ? added : activate(added, open.active);
  return focusPane(fronted, arrangement.focus);
};

/**
 * What was kept before Open and Deferred were two tabs has no Deferred tab, and follow-ups stand nowhere else. Given one once, so a person who closes the tab
 * afterwards keeps it closed.
 */
export function giveKeptPanesDeferred(): void {
  if (typeof window === "undefined") return;
  if (readKept(DEFERRED_GIVEN, (stored) => stored === true) === true) return;
  for (const key of Object.keys(window.localStorage)) {
    if (!key.startsWith(`armada:kept:${JOB}`)) continue;
    const name = key.slice("armada:kept:".length);
    try {
      const entry = readKept<Entry>(name, reviveEntry);
      if (entry !== undefined)
        writeKept(name, {
          ...entry,
          arrangement: withDeferred(entry.arrangement),
        });
    } catch {
      // Not a Job's panes any more: `keptEntry` sets it aside the same way.
    }
  }
  const shared = readKept(SHARED, reviveShared);
  if (shared !== undefined)
    writeKept(SHARED, {
      ...shared,
      base: shared.base === undefined ? undefined : withDeferred(shared.base),
      saved: Object.fromEntries(
        Object.entries(shared.saved).map(([name, one]) => [
          name,
          withDeferred(one),
        ]),
      ),
    });
  writeKept(DEFERRED_GIVEN, true);
}

/** A saved request, as the page a Requests tab shows: its collection and its id. */
export const savedPage = (collection: string, request: string): string =>
  JSON.stringify([
    collection,
    request,
  ]);

/** The collection and request a Requests tab's page names, or `undefined` for a tab with none. */
export function savedOfPage(page: string | undefined):
  | {
      collection: string;
      request: string;
    }
  | undefined {
  if (page === undefined) return undefined;
  try {
    const parsed: unknown = JSON.parse(page);
    return Array.isArray(parsed) && typeof parsed[0] === "string" && typeof parsed[1] === "string"
      ? {
          collection: parsed[0],
          request: parsed[1],
        }
      : undefined;
  } catch {
    return undefined;
  }
}

export type JobPanes = {
  arrangement: Arrangement;
  layout: string;
  edited: boolean;
  /** Every layout there is, Default first. */
  layouts: readonly string[];
  forAll: string;
};

type FamilySpec = {
  /** Which family, as the Layout control names it. */
  name: "job" | "pr";
  /** The key its layouts are kept under, and the prefix of each owner's own. */
  shared: string;
  owner: string;
  /** The kinds a tab of this family can be, and its + menu offers. */
  kinds: readonly TabKind[];
  /** Default, until it is updated. */
  arrangement: () => Arrangement;
  /** The Layout control's last row: the layout new owners of the family open on. */
  forAllLabel: string;
  /** What is done to kept panes before the family first reads them. */
  migrate?: () => void;
};

/** A family of owners, with its own layouts, kept under keys of its own. */
export function createPaneFamily(spec: FamilySpec) {
  const OWNER = spec.owner;
  const revive = reviverOf(spec.kinds);
  const reviveFamily = sharedReviverOf(spec.kinds);
  const empty = (): State => ({
    jobs: {},
    saved: {},
    forAll: DEFAULT_LAYOUT,
    versions: {},
    base: undefined,
  });

  const initial = (): State => {
    spec.migrate?.();
    return {
      ...empty(),
      ...readKept(spec.shared, reviveFamily),
    };
  };

  /** An owner's own panes as they were kept, where there are any. */
  const keptEntry = (jobId: string): Entry | undefined => {
    try {
      return readKept<Entry>(OWNER + jobId, revive);
    } catch {
      return undefined;
    }
  };

  /** What changed between two states is written: the shared part once, and each owner whose entry is not the one it was. */
  function keep(was: State, next: State): void {
    if (
      next.saved !== was.saved ||
      next.forAll !== was.forAll ||
      next.versions !== was.versions ||
      next.base !== was.base
    )
      writeKept(spec.shared, {
        saved: next.saved,
        forAll: next.forAll,
        versions: next.versions,
        base: next.base,
      });
    for (const [id, entry] of Object.entries(next.jobs))
      if (was.jobs[id] !== entry) writeKept(OWNER + id, entry);
  }

  let state: State = initial();
  const listeners = new Set<() => void>();
  const told = () => listeners.forEach((hear) => hear());

  function set(next: State): void {
    const was = state;
    state = next;
    keep(was, next);
    told();
  }

  /** Everything kept of the family's panes, gone, and the window as it opens with none. */
  function forget(): void {
    forgetKept(spec.shared);
    for (const key of Object.keys(window.localStorage))
      if (key.startsWith(`armada:kept:${OWNER}`)) forgetKept(key.slice("armada:kept:".length));
    histories.clear();
    areas.clear();
    state = empty();
    told();
  }

  /** Saved layouts a mock offers beside Default, in place of any a person made. */
  function seedLayouts(saved: Record<string, Arrangement>): void {
    set({
      ...state,
      saved,
    });
  }

  const versionOf = (name: string): number => state.versions[name] ?? 0;

  const layoutNamed = (name: string): Arrangement =>
    copyOf((name === DEFAULT_LAYOUT ? state.base : state.saved[name]) ?? spec.arrangement());

  /** The owner's entry, made on first sight from the layout for all. It is brought to that layout while the person has not chosen otherwise, and to a layout's update while the owner is not edited. No notice: the same answer every time. */
  function ensure(jobId: string): Entry {
    const held = state.jobs[jobId] ?? keptEntry(jobId);
    if (held !== undefined) {
      const following = !held.chosen && held.layout !== state.forAll;
      const stale = !held.edited && held.version !== versionOf(held.layout);
      if (!following && !stale) {
        if (state.jobs[jobId] === undefined)
          state = {
            ...state,
            jobs: {
              ...state.jobs,
              [jobId]: held,
            },
          };
        return held;
      }
    }
    const layout = held?.chosen === true ? held.layout : state.forAll;
    const made: Entry = {
      arrangement: layoutNamed(layout),
      layout,
      edited: false,
      chosen: held?.chosen ?? false,
      version: versionOf(layout),
    };
    state = {
      ...state,
      jobs: {
        ...state.jobs,
        [jobId]: made,
      },
    };
    return made;
  }

  const subscribe = (hear: () => void) => {
    listeners.add(hear);
    return () => void listeners.delete(hear);
  };

  function useJobPanes(jobId: string): JobPanes {
    const seen = useSyncExternalStore(subscribe, () => (ensure(jobId), state));
    const entry = seen.jobs[jobId]!;
    return {
      arrangement: entry.arrangement,
      layout: entry.layout,
      edited: entry.edited,
      layouts: [
        DEFAULT_LAYOUT,
        ...Object.keys(seen.saved),
      ],
      forAll: seen.forAll,
    };
  }

  function put(jobId: string, change: (entry: Entry) => Entry): void {
    const entry = ensure(jobId);
    set({
      ...state,
      jobs: {
        ...state.jobs,
        [jobId]: change(entry),
      },
    });
  }

  /** Changes whenever any owner's panes do, for what is read from them without opening the owner. */
  function usePanesVersion(): unknown {
    return useSyncExternalStore(subscribe, () => state);
  }

  /** The owners whose panes hold a Custom tab of this mod, open in this window or kept from an earlier one. */
  function jobsHoldingMod(mod: string): string[] {
    const kept = Object.keys(window.localStorage)
      .filter((key) => key.startsWith(`armada:kept:${OWNER}`))
      .map((key) => key.slice(`armada:kept:${OWNER}`.length));
    return [
      ...new Set([
        ...Object.keys(state.jobs),
        ...kept,
      ]),
    ].filter((id) => {
      const entry = state.jobs[id] ?? keptEntry(id);
      return (
        entry !== undefined &&
        panesOf(entry.arrangement.root).some((pane) =>
          pane.tabs.some((tab) => tab.kind === "custom" && tab.mod === mod),
        )
      );
    });
  }

  /** The owner's arrangement as it stands now. */
  const peek = (jobId: string): Arrangement => ensure(jobId).arrangement;

  // ---- Where the person has been, for ⌘Z --------------------------------------------------------------------

  type History = {
    past: Entry[];
    future: Entry[];
    merge?: string;
    at: number;
  };
  const histories = new Map<string, History>();
  const HISTORY = 50;
  /** A run of one kind of change, a drag along a line, is one step back. */
  const MERGE_MS = 600;

  function remember(jobId: string, was: Entry, merge?: string): void {
    const history = histories.get(jobId) ?? {
      past: [],
      future: [],
      at: 0,
    };
    const now = Date.now();
    const joins = merge !== undefined && history.merge === merge && now - history.at < MERGE_MS;
    history.at = now;
    if (merge !== undefined) history.merge = merge;
    else delete history.merge;
    if (!joins)
      history.past = [
        ...history.past.slice(-(HISTORY - 1)),
        was,
      ];
    history.future = [];
    histories.set(jobId, history);
  }

  /** The area's size in px as drawn, which the panes' minimums are held against. */
  const areas = new Map<
    string,
    {
      w: number;
      h: number;
    }
  >();
  function measured(
    jobId: string,
    size: {
      w: number;
      h: number;
    },
  ): void {
    areas.set(jobId, size);
  }

  /** Whether a change leaves every pane its minimum, or in an area too small for that, no pane further short than before. */
  function allowed(jobId: string, was: Arrangement, next: Arrangement): boolean {
    const size = areas.get(jobId);
    return size === undefined || size.w === 0 || shortfall(next, size) <= shortfall(was, size);
  }

  /** A change of arrangement the person made: this owner is edited. One that would squeeze a pane below its minimum is refused, unless it is forced, as a Job opening a tab is. `merge` names a run of changes that is one step for ⌘Z. */
  function rearrange(
    jobId: string,
    change: (arrangement: Arrangement) => Arrangement,
    merge?: string,
    force = false,
  ): void {
    const entry = ensure(jobId);
    const next = change(entry.arrangement);
    if (next === entry.arrangement || (!force && !allowed(jobId, entry.arrangement, next))) return;
    remember(jobId, entry, merge);
    set({
      ...state,
      jobs: {
        ...state.jobs,
        [jobId]: {
          ...entry,
          arrangement: next,
          edited: true,
          chosen: true,
        },
      },
    });
  }

  /** Where a tab dragged over a pane would land, and whether the pane can take it. */
  function droppable(jobId: string, tabId: string, paneId: string, zone: Side | "middle"): boolean {
    const was = ensure(jobId).arrangement;
    const next =
      zone === "middle"
        ? moveTabToPane(was, tabId, paneId)
        : moveTabToEdge(was, tabId, paneId, zone);
    return next !== was && allowed(jobId, was, next);
  }

  /** Whether a tab dragged over the gap of a split can be dropped there: a new pane between the two sides, with room for every pane. */
  function droppableGap(jobId: string, tabId: string, splitId: string): boolean {
    const was = ensure(jobId).arrangement;
    const next = moveTabToGap(was, tabId, splitId);
    return next !== was && allowed(jobId, was, next);
  }

  /** Whether a tab dragged over an outer edge of the area can be dropped there: a new pane along the whole edge that changes the layout and leaves every pane its minimum. */
  function droppableEdge(jobId: string, tabId: string, side: Side): boolean {
    const was = ensure(jobId).arrangement;
    const next = moveTabToEnd(was, tabId, side);
    return next !== was && allowed(jobId, was, next);
  }

  function go(jobId: string, from: "past" | "future"): void {
    const history = histories.get(jobId);
    const target = history?.[from].pop();
    if (history === undefined || target === undefined) return;
    const entry = ensure(jobId);
    (from === "past" ? history.future : history.past).push(entry);
    delete history.merge;
    set({
      ...state,
      jobs: {
        ...state.jobs,
        [jobId]: target,
      },
    });
  }

  const undo = (jobId: string): void => go(jobId, "past");
  const redo = (jobId: string): void => go(jobId, "future");

  /** Which tab is in front and which pane has the focus: where the person is, not how the area is laid out. */
  function look(jobId: string, change: (arrangement: Arrangement) => Arrangement): void {
    put(jobId, (entry) => {
      const next = change(entry.arrangement);
      return next === entry.arrangement
        ? entry
        : {
            ...entry,
            arrangement: next,
          };
    });
  }

  /** A tab the owner opened itself. One that joins a pane is content and leaves the layout as it was; one that makes a split is arrangement. */
  function jobOpens(
    jobId: string,
    kind: TabKind,
    label?: string,
    page?: string,
    mod?: string,
  ): void {
    if (hasKind(peek(jobId), kind))
      look(jobId, (arrangement) => arrive(arrangement, kind, label, page, mod));
    else
      rearrange(
        jobId,
        (arrangement) => arrive(arrangement, kind, label, page, mod),
        undefined,
        true,
      );
  }

  /** A shell the Job opened itself, the agent's among them, gets a tab behind the one in front and the focus stays where it was. One that has a tab already is left alone. `first` is the Job's first shell, which a tab with no ID draws. */
  function jobOpensTerminal(jobId: string, terminal: string | null, first: boolean): void {
    const arrangement = peek(jobId);
    const has = panesOf(arrangement.root).some((one) =>
      one.tabs.some(
        (tab) =>
          tab.kind === "terminal" &&
          ((terminal !== null && tab.terminal === terminal) ||
            (first && tab.terminal === undefined)),
      ),
    );
    if (has) return;
    const change = (now: Arrangement) => arriveTerminalBehind(now, terminal ?? undefined);
    if (hasKind(arrangement, "terminal")) look(jobId, change);
    else rearrange(jobId, change, undefined, true);
  }

  /** A shell the Job's agent showed comes to the front of its pane, or opens in front where it has no tab, and the focus stays where it was. `first` is as `jobOpensTerminal`'s. */
  function jobShowsTerminal(jobId: string, terminal: string | null, first: boolean): void {
    const arrangement = peek(jobId);
    const open = panesOf(arrangement.root)
      .flatMap((one) => one.tabs)
      .find(
        (tab) =>
          tab.kind === "terminal" &&
          ((terminal !== null && tab.terminal === terminal) ||
            (first && tab.terminal === undefined)),
      );
    const focusKept = (now: Arrangement, next: Arrangement): Arrangement =>
      next === now
        ? now
        : {
            ...next,
            focus: now.focus,
          };
    if (open !== undefined) {
      look(jobId, (now) => focusKept(now, activate(now, open.id)));
      return;
    }
    const change = (now: Arrangement) =>
      focusKept(
        now,
        arrive(now, "terminal", undefined, undefined, undefined, undefined, terminal ?? undefined),
      );
    if (hasKind(arrangement, "terminal")) look(jobId, change);
    else rearrange(jobId, change, undefined, true);
  }

  /** A saved request opens in the Job's Requests tab: the one there is comes to the front on it, and with none a tab opens. */
  function openSavedRequest(jobId: string, collection: string, request: string): void {
    const page = savedPage(collection, request);
    const holder = holderOf(peek(jobId), "request");
    const open = holder?.tabs.find((one) => one.kind === "request");
    if (open === undefined) return jobOpens(jobId, "request", undefined, page);
    look(jobId, (arrangement) => showPage(arrangement, open.id, page));
  }

  function takeLayout(jobId: string, name: string): void {
    remember(jobId, ensure(jobId));
    put(jobId, () => ({
      arrangement: layoutNamed(name),
      layout: name,
      edited: false,
      chosen: true,
      version: versionOf(name),
    }));
  }

  /** The owner's arrangement becomes what the layout is, for every owner on it that is not edited. */
  function updateLayout(jobId: string): void {
    const entry = ensure(jobId);
    if (!entry.edited) return;
    const version = versionOf(entry.layout) + 1;
    const kept = copyOf(entry.arrangement);
    set({
      ...state,
      versions: {
        ...state.versions,
        [entry.layout]: version,
      },
      ...(entry.layout === DEFAULT_LAYOUT
        ? {
            base: kept,
          }
        : {
            saved: {
              ...state.saved,
              [entry.layout]: kept,
            },
          }),
      jobs: {
        ...state.jobs,
        [jobId]: {
          ...entry,
          edited: false,
          version,
        },
      },
    });
  }

  /** A name is free when no layout of the family has it. */
  const nameTaken = (name: string): boolean =>
    name.trim() === DEFAULT_LAYOUT || state.saved[name.trim()] !== undefined;

  function saveLayoutAs(jobId: string, name: string): void {
    const entry = ensure(jobId);
    const label = name.trim();
    if (label === "" || label === DEFAULT_LAYOUT) return;
    set({
      ...state,
      saved: {
        ...state.saved,
        [label]: copyOf(entry.arrangement),
      },
      jobs: {
        ...state.jobs,
        [jobId]: {
          ...entry,
          layout: label,
          edited: false,
          chosen: true,
          version: 0,
        },
      },
    });
  }

  function resetLayout(jobId: string): void {
    const entry = ensure(jobId);
    remember(jobId, entry);
    put(jobId, () => ({
      ...entry,
      arrangement: layoutNamed(entry.layout),
      edited: false,
      version: versionOf(entry.layout),
    }));
  }

  /** The named layout is the one new owners of the family open on from now on, and so is every one that has not been given another. */
  function setLayoutForAll(layout: string): void {
    set({
      ...state,
      forAll: layout,
    });
  }

  return {
    name: spec.name,
    kinds: spec.kinds,
    forAllLabel: spec.forAllLabel,
    forget,
    seedLayouts,
    useJobPanes,
    usePanesVersion,
    jobsHoldingMod,
    peek,
    measured,
    rearrange,
    droppable,
    droppableGap,
    droppableEdge,
    undo,
    redo,
    look,
    jobOpens,
    jobOpensTerminal,
    jobShowsTerminal,
    openSavedRequest,
    takeLayout,
    updateLayout,
    nameTaken,
    saveLayoutAs,
    resetLayout,
    setLayoutForAll,
  };
}

export type PaneFamily = ReturnType<typeof createPaneFamily>;

/** The Jobs' panes, under the keys they have always been kept under. */
export const jobPanes: PaneFamily = createPaneFamily({
  name: "job",
  shared: SHARED,
  owner: JOB,
  kinds: JOB_TAB_KINDS,
  arrangement: defaultArrangement,
  forAllLabel: "New Jobs open on",
  migrate: giveKeptPanesDeferred,
});

/** The pull requests' panes on the Reviews page, kept apart from the Jobs'. */
export const prPanes: PaneFamily = createPaneFamily({
  name: "pr",
  shared: "pr-panes:shared",
  owner: "pr-panes:pr:",
  kinds: PR_TAB_KINDS,
  arrangement: prArrangement,
  forAllLabel: "New pull requests open on",
});

/** The family the panes drawn here belong to: a Job's, unless a pull request's area says otherwise. */
export const PaneFamilyContext = createContext<PaneFamily>(jobPanes);
export const usePaneFamily = (): PaneFamily => useContext(PaneFamilyContext);

/** Everything kept of the panes of every family, gone, and the window as it opens with none: a test or a mock starts from here. */
export function forgetPanes(): void {
  forgetKept(DEFERRED_GIVEN);
  jobPanes.forget();
  prPanes.forget();
}

export const {
  seedLayouts,
  useJobPanes,
  usePanesVersion,
  jobsHoldingMod,
  peek,
  measured,
  rearrange,
  droppable,
  droppableGap,
  droppableEdge,
  undo,
  redo,
  look,
  jobOpens,
  jobOpensTerminal,
  jobShowsTerminal,
  openSavedRequest,
  takeLayout,
  updateLayout,
  nameTaken,
  saveLayoutAs,
  resetLayout,
  setLayoutForAll,
} = jobPanes;
