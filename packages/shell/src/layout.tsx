// Layout: which Dashboard tabs and panels, Job detail tabs and rail rows Bridge draws, and in what
// order. `docs/concepts/layout-mods.md`.
//
// The registry is the one list of what can be arranged. A mod's `layout.json` and the owner's own
// choice name entries by id and nothing else, so an id that is gone is skipped and never fatal.
// Nothing here draws: a surface reads `useLayout(region)` and draws what it is handed.

import { createContext, useContext, useMemo, useSyncExternalStore } from "react";
import {
  Activity,
  Bot,
  CircleCheck,
  CircleDot,
  GitMerge,
  LayoutDashboard,
  ListTodo,
  LoaderCircle,
  ScrollText,
  Send,
  Settings,
  ShieldCheck,
  Waypoints,
  Workflow,
  type LucideIcon,
} from "lucide-react";

import { RAIL_PANELS, SURFACE, SURFACES } from "./surfaces";

export type LayoutRegion = "dashboard.tabs" | "dashboard.panels" | "job.tabs" | "rail";

export type LayoutEntry = {
  id: string;
  label: string;
  /** The registry glyph, `packages/icons/icons/`. */
  icon: LucideIcon;
  /** False for anything the owner has to decide on, or has to reach to undo a layout. */
  hideable: boolean;
  /** Where on the Dashboard a panel stands: above the tab strip, or on the board. */
  slot?: "top" | "board";
};

type Spec = {
  label: string;
  /** Whether a layout may reorder it. The rail's order is the shell's own. */
  ordered: boolean;
  /** Whether a layout may say which entry opens first. */
  firstable: boolean;
  entries: readonly LayoutEntry[];
};

const RAIL_FIXED: readonly string[] = [SURFACE.overview, SURFACE.settings, SURFACE.mods];

/** Default order is the array's own, and every entry is drawn until a layout says otherwise. */
export const LAYOUT: Readonly<Record<LayoutRegion, Spec>> = {
  "dashboard.tabs": {
    label: "Dashboard tabs",
    ordered: true,
    firstable: true,
    entries: [
      { id: "command-central", label: "Command Central", icon: CircleDot, hideable: false },
      { id: "running", label: "Running", icon: LoaderCircle, hideable: true },
      { id: "done", label: "Done", icon: CircleCheck, hideable: true },
    ],
  },
  "dashboard.panels": {
    label: "Dashboard panels",
    ordered: true,
    firstable: false,
    entries: [
      { id: "quick-dispatch", label: "Dispatch", icon: Send, hideable: true, slot: "top" },
      { id: "fleet", label: "Fleet", icon: Waypoints, hideable: false, slot: "board" },
      { id: "merge-line", label: "Merge line", icon: GitMerge, hideable: true, slot: "board" },
    ],
  },
  "job.tabs": {
    label: "Job tabs",
    ordered: true,
    firstable: true,
    entries: [
      { id: "overview", label: "Overview", icon: LayoutDashboard, hideable: false },
      { id: "workflow", label: "Workflow", icon: Workflow, hideable: true },
      { id: "plan", label: "Plan", icon: ListTodo, hideable: false },
      { id: "record", label: "Record", icon: ScrollText, hideable: true },
      { id: "checks", label: "Checks", icon: ShieldCheck, hideable: true },
      { id: "drones", label: "Drones", icon: Bot, hideable: true },
      { id: "pulse", label: "Pulse", icon: Activity, hideable: true },
      { id: "settings", label: "Settings", icon: Settings, hideable: true },
    ],
  },
  rail: {
    label: "Rail",
    ordered: false,
    firstable: false,
    entries: RAIL_PANELS.flatMap((panel) => panel.surfaces).flatMap((id) => {
      const surface = SURFACES.find((one) => one.id === id);
      return surface === undefined ? [] : [{ id, label: surface.label, icon: surface.icon, hideable: !RAIL_FIXED.includes(id) }];
    }),
  },
};

export const LAYOUT_REGIONS = Object.keys(LAYOUT) as LayoutRegion[];

/** What one layer says about one region. Every field is optional, and a field replaces the one below it. */
export type RegionChoice = { order?: readonly string[]; hidden?: readonly string[]; first?: string };

/** `layout.json` as read, and the owner's own choice as kept. */
export type LayoutFile = { regions: Partial<Record<LayoutRegion, RegionChoice>> };

const VERSION = 1;
export const LAYOUT_BYTES = 4096;
const MOST_IDS = 32;
const ID = /^[a-z][a-z0-9-]{0,39}$/;

export type Parsed = { file: LayoutFile; problems: string[]; ignored: string[] };

/**
 * `layout.json` checked. **Only known keys, ids and shapes pass**: a wrong type or an extra key is a
 * problem and the file is refused; a region or an id this build does not have is ignored and named.
 */
export function parseLayout(text: string): Parsed {
  const problems: string[] = [];
  const ignored: string[] = [];
  const file: LayoutFile = { regions: {} };
  if (new TextEncoder().encode(text).length > LAYOUT_BYTES) return { file, problems: [`larger than ${LAYOUT_BYTES} bytes`], ignored };
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch {
    return { file, problems: ["not JSON"], ignored };
  }
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return { file, problems: ["not an object"], ignored };
  const top = raw as Record<string, unknown>;
  if (top["version"] !== VERSION) problems.push(`version is not ${VERSION}`);
  for (const [key, value] of Object.entries(top)) {
    if (key === "version") continue;
    const region = LAYOUT_REGIONS.find((one) => one === key);
    if (region === undefined) {
      ignored.push(key);
      continue;
    }
    const spec = LAYOUT[region];
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      problems.push(`${key} is not an object`);
      continue;
    }
    const choice: { order?: string[]; hidden?: string[]; first?: string } = {};
    for (const [field, body] of Object.entries(value as Record<string, unknown>)) {
      if (field === "order" || field === "hidden") {
        const ids = idsOf(body);
        if (ids === undefined) problems.push(`${key}.${field} is not a list of up to ${MOST_IDS} ids`);
        else choice[field] = known(spec, ids, `${key}.${field}`, ignored);
      } else if (field === "first") {
        if (typeof body !== "string" || !ID.test(body)) problems.push(`${key}.first is not an id`);
        else if (!spec.entries.some((one) => one.id === body)) ignored.push(`${key}.first ${body}`);
        else choice.first = body;
      } else problems.push(`${key}.${field} is not a field`);
    }
    if (!spec.ordered && choice.order !== undefined) {
      delete choice.order;
      ignored.push(`${key}.order`);
    }
    if (!spec.firstable && choice.first !== undefined) {
      delete choice.first;
      ignored.push(`${key}.first`);
    }
    file.regions[region] = choice;
  }
  return { file: problems.length === 0 ? file : { regions: {} }, problems, ignored };
}

function idsOf(body: unknown): string[] | undefined {
  if (!Array.isArray(body) || body.length > MOST_IDS) return undefined;
  return body.every((one): one is string => typeof one === "string" && ID.test(one)) ? body : undefined;
}

function known(spec: Spec, ids: string[], at: string, ignored: string[]): string[] {
  const seen = new Set<string>();
  return ids.filter((id) => {
    if (!spec.entries.some((one) => one.id === id)) ignored.push(`${at} ${id}`);
    else if (!seen.has(id)) return seen.add(id), true;
    return false;
  });
}

export type Layer = { by: string; file: LayoutFile };
export type Placed = LayoutEntry & {
  visible: boolean;
  /** Who hid it: a mod's title, or `You`. */
  hiddenBy?: string;
};
export type Resolved = {
  /** Every entry, in order, hidden ones included. */
  all: readonly Placed[];
  /** What is drawn, in order. */
  shown: readonly Placed[];
  /** The entry that opens first. */
  first: string;
};

/**
 * The shipped layout, then each layer over it in the order given. **A field a layer sets replaces the
 * same field below it**, an entry that cannot be hidden is never hidden, and `first` is never hidden.
 */
export function resolveLayout(region: LayoutRegion, layers: readonly Layer[]): Resolved {
  const spec = LAYOUT[region];
  let order = spec.entries.map((one) => one.id);
  let hidden = new Map<string, string>();
  let first = order[0] ?? "";
  for (const { by, file } of layers) {
    const choice = file.regions[region];
    if (choice === undefined) continue;
    if (choice.order !== undefined && spec.ordered) {
      const named = choice.order.filter((id) => order.includes(id));
      order = [...named, ...order.filter((id) => !named.includes(id))];
    }
    if (choice.hidden !== undefined) {
      hidden = new Map(choice.hidden.filter((id) => spec.entries.find((one) => one.id === id)?.hideable === true).map((id) => [id, by]));
    }
    if (choice.first !== undefined && spec.firstable) first = choice.first;
  }
  const all = order.map((id): Placed => {
    const entry = spec.entries.find((one) => one.id === id)!;
    const by = hidden.get(id);
    return by === undefined ? { ...entry, visible: true } : { ...entry, visible: false, hiddenBy: by };
  });
  const shown = all.filter((one) => one.visible);
  return { all, shown, first: shown.some((one) => one.id === first) ? first : (shown[0]?.id ?? first) };
}

/** A layout mod: `layout.json` in a mod folder, once Fleet has checked it. */
export type LayoutMod = {
  name: string;
  title: string;
  /** This machine's switch for the mod. */
  enabled: boolean;
  /** Why Bridge may not apply it, as Fleet said it. */
  problem?: string;
  /** The branch of the repository it was put on, once it has been. */
  branch?: string;
  file: LayoutFile;
};

export type LayoutState = { mods: readonly LayoutMod[]; own: LayoutFile };

export interface LayoutSource {
  get(): LayoutState;
  subscribe(on: () => void): () => void;
  /** The owner's own choice for one region, over every mod. `undefined` takes his choice for it back. */
  setOwn(region: LayoutRegion, choice: RegionChoice | undefined): void;
  /** The shipped layout: his own choices cleared and every layout mod switched off. The mods stay installed. */
  toDefaults(): void;
  setEnabled(name: string, enabled: boolean): void;
  promote(name: string): void;
}

const EMPTY: LayoutState = { mods: [], own: { regions: {} } };

/** A source held in memory: the mock runs on it, and a window with no mods runs on the empty one. */
export function createLayoutSource(): LayoutSource & {
  /** A mod arriving or being rewritten, the way a Session writing `layout.json` would. */
  install(mod: LayoutMod): void;
  reset(): void;
} {
  let held = EMPTY;
  const listeners = new Set<() => void>();
  const set = (next: LayoutState) => {
    held = next;
    listeners.forEach((on) => on());
  };
  const edit = (name: string, change: Partial<LayoutMod>) => set({ ...held, mods: held.mods.map((mod) => (mod.name === name ? { ...mod, ...change } : mod)) });
  return {
    get: () => held,
    subscribe: (on) => {
      listeners.add(on);
      return () => void listeners.delete(on);
    },
    setOwn: (region, choice) => {
      const { [region]: _was, ...rest } = held.own.regions;
      set({ ...held, own: { regions: choice === undefined ? rest : { ...rest, [region]: choice } } });
    },
    toDefaults: () => set({ mods: held.mods.map((mod) => ({ ...mod, enabled: false })), own: EMPTY.own }),
    setEnabled: (name, enabled) => edit(name, { enabled }),
    promote: (name) => edit(name, { branch: `armada/mod-${name}-${Date.now()}` }),
    install: (mod) => set({ ...held, mods: held.mods.some((one) => one.name === mod.name) ? held.mods.map((one) => (one.name === mod.name ? { ...mod, branch: one.branch } : one)) : [...held.mods, mod] }),
    reset: () => set(EMPTY),
  };
}

const LayoutSourceContext = createContext<LayoutSource>(createLayoutSource());

export const LayoutSourceProvider = LayoutSourceContext.Provider;

/** The source in force, and its state as it stands. */
export function useLayouts(): LayoutState & { source: LayoutSource } {
  const source = useContext(LayoutSourceContext);
  const state = useSyncExternalStore(source.subscribe, source.get);
  return { ...state, source };
}

/** The layers in force, lowest first: each mod that is on and passes, by name, then the owner's own. */
export function layersOf(state: LayoutState): Layer[] {
  return [...state.mods.filter((mod) => mod.enabled && mod.problem === undefined).sort((a, b) => a.name.localeCompare(b.name)).map((mod) => ({ by: mod.title, file: mod.file })), { by: "You", file: state.own }];
}

/** One region as it is to be drawn. */
export function useLayout(region: LayoutRegion): Resolved {
  const state = useLayouts();
  return useMemo(() => resolveLayout(region, layersOf(state)), [region, state.mods, state.own]); // eslint-disable-line react-hooks/exhaustive-deps
}
