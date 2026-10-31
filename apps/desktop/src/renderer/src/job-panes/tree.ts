// The open Job's area as a tree of panes. A split holds two nodes side by side (`row`) or one over the other (`column`) at a ratio; a pane holds tabs, one
// of them active. Every operation here is pure: it takes an arrangement and returns the next one, so the store, the keys and the menus share one set of rules.
// Tab, pane and split ids come from the arrangement's own counter, so two Jobs never need to agree on any.

export const TAB_KINDS = ["thread", "browser", "terminal", "done", "now", "open", "plan", "changes", "pulls", "issues", "jobs", "ledger", "files", "request", "system", "bays", "spine"] as const;
export type TabKind = (typeof TAB_KINDS)[number];

/** `label` is what the tab reads where its kind alone would not tell two apart: a terminal's shell. `page` names the page a browser tab shows where the tab's own id does not (a page the Job opened itself). */
export type Tab = { id: string; kind: TabKind; label?: string; page?: string };
export type Pane = { type: "pane"; id: string; tabs: readonly Tab[]; active: string | undefined };
export type Split = { type: "split"; id: string; dir: "row" | "column"; ratio: number; a: Node; b: Node };
export type Node = Pane | Split;

export type Arrangement = {
  root: Node;
  /** The pane keys and the menus act on. */
  focus: string;
  /** The next number for an id. */
  next: number;
};

export type Rect = { x: number; y: number; w: number; h: number };
export type Side = "left" | "right" | "top" | "bottom";
export type Toward = "left" | "right" | "up" | "down";

/** The least a pane keeps, in px: wide enough for its strip and tall enough for a few lines. */
export const MIN_PANE_W = 160;
export const MIN_PANE_H = 120;

/** The least a node needs along an axis: a pane its minimum, two sides along the split the two together, across it the larger. */
export function minExtent(node: Node, axis: "row" | "column"): number {
  if (node.type === "pane") return axis === "row" ? MIN_PANE_W : MIN_PANE_H;
  const [a, b] = [minExtent(node.a, axis), minExtent(node.b, axis)];
  return node.dir === axis ? a + b : Math.max(a, b);
}

/** How far the panes fall short of their minimums in an area of this size, in px; 0 when every one keeps its own. */
export function shortfall(arrangement: Arrangement, size: { w: number; h: number }): number {
  let total = 0;
  for (const rect of rectsOf(arrangement.root).panes.values()) total += Math.max(0, MIN_PANE_W - rect.w * size.w - 0.5) + Math.max(0, MIN_PANE_H - rect.h * size.h - 0.5);
  return total;
}

/** The least a side of a split keeps, as a share of the split. */
export const RATIO_MIN = 0.12;
const clampRatio = (ratio: number): number => Math.min(1 - RATIO_MIN, Math.max(RATIO_MIN, ratio));

// ---- Reading ---------------------------------------------------------------------------------------------

export function panesOf(node: Node): Pane[] {
  return node.type === "pane" ? [node] : [...panesOf(node.a), ...panesOf(node.b)];
}

export function paneById(arrangement: Arrangement, id: string): Pane | undefined {
  return panesOf(arrangement.root).find((one) => one.id === id);
}

export function paneOfTab(arrangement: Arrangement, tabId: string): Pane | undefined {
  return panesOf(arrangement.root).find((one) => one.tabs.some((tab) => tab.id === tabId));
}

export function tabById(arrangement: Arrangement, tabId: string): Tab | undefined {
  return panesOf(arrangement.root)
    .flatMap((one) => one.tabs)
    .find((tab) => tab.id === tabId);
}

/** The pane that holds a tab of this kind, the focused one first. */
export function holderOf(arrangement: Arrangement, kind: TabKind): Pane | undefined {
  const holding = panesOf(arrangement.root).filter((one) => one.tabs.some((tab) => tab.kind === kind));
  return holding.find((one) => one.id === arrangement.focus) ?? holding[0];
}

export function hasKind(arrangement: Arrangement, kind: TabKind): boolean {
  return holderOf(arrangement, kind) !== undefined;
}

/** Each pane's box as shares of the whole, and each split's, for the handle on the line between its two sides. */
export function rectsOf(root: Node, within: Rect = { x: 0, y: 0, w: 1, h: 1 }): { panes: Map<string, Rect>; splits: { id: string; dir: "row" | "column"; ratio: number; rect: Rect; a: Node; b: Node }[] } {
  const panes = new Map<string, Rect>();
  const splits: { id: string; dir: "row" | "column"; ratio: number; rect: Rect; a: Node; b: Node }[] = [];
  const walk = (node: Node, rect: Rect) => {
    if (node.type === "pane") return void panes.set(node.id, rect);
    splits.push({ id: node.id, dir: node.dir, ratio: node.ratio, rect, a: node.a, b: node.b });
    if (node.dir === "row") {
      const first = rect.w * node.ratio;
      walk(node.a, { ...rect, w: first });
      walk(node.b, { ...rect, x: rect.x + first, w: rect.w - first });
    } else {
      const first = rect.h * node.ratio;
      walk(node.a, { ...rect, h: first });
      walk(node.b, { ...rect, y: rect.y + first, h: rect.h - first });
    }
  };
  walk(root, within);
  return { panes, splits };
}

/** The pane next to this one in a direction: the nearest that shares some of its edge. */
export function neighbourOf(arrangement: Arrangement, paneId: string, toward: Toward): string | undefined {
  const { panes } = rectsOf(arrangement.root);
  const from = panes.get(paneId);
  if (from === undefined) return undefined;
  const eps = 1e-6;
  const overlap = (a0: number, a1: number, b0: number, b1: number) => Math.min(a1, b1) - Math.max(a0, b0);
  let best: { id: string; gap: number; shared: number } | undefined;
  for (const [id, rect] of panes) {
    if (id === paneId) continue;
    const gap = toward === "left" ? from.x - (rect.x + rect.w) : toward === "right" ? rect.x - (from.x + from.w) : toward === "up" ? from.y - (rect.y + rect.h) : rect.y - (from.y + from.h);
    const shared = toward === "left" || toward === "right" ? overlap(from.y, from.y + from.h, rect.y, rect.y + rect.h) : overlap(from.x, from.x + from.w, rect.x, rect.x + rect.w);
    if (gap < -eps || shared <= eps) continue;
    if (best === undefined || gap < best.gap - eps || (Math.abs(gap - best.gap) <= eps && shared > best.shared)) best = { id, gap, shared };
  }
  return best?.id;
}

// ---- Building --------------------------------------------------------------------------------------------

const mint = (arrangement: Arrangement, prefix: "p" | "s" | "t"): [string, Arrangement] => [`${prefix}${arrangement.next}`, { ...arrangement, next: arrangement.next + 1 }];

type Shape = TabKind[] | { dir: "row" | "column"; ratio: number; a: Shape; b: Shape; front?: TabKind };

/** An arrangement from a plain shape: a list of kinds is a pane, with `front` the tab shown. */
export function build(shape: Shape, front?: TabKind, labels: Partial<Record<TabKind, string>> = {}): Arrangement {
  let at: Arrangement = { root: { type: "pane", id: "p0", tabs: [], active: undefined }, focus: "p0", next: 1 };
  const make = (one: Shape): Node => {
    if (Array.isArray(one)) {
      const [id, afterPane] = mint(at, "p");
      at = afterPane;
      const tabs: Tab[] = [];
      for (const kind of one) {
        const [tabId, afterTab] = mint(at, "t");
        at = afterTab;
        tabs.push(tabOf(tabId, kind, labels[kind]));
      }
      const active = (front === undefined ? undefined : tabs.find((tab) => tab.kind === front)) ?? tabs[0];
      return { type: "pane", id, tabs, active: active?.id };
    }
    const [id, afterSplit] = mint(at, "s");
    at = afterSplit;
    return { type: "split", id, dir: one.dir, ratio: clampRatio(one.ratio), a: make(one.a), b: make(one.b) };
  };
  const root = make(shape);
  at = { ...at, root };
  return { ...at, focus: panesOf(root)[0]!.id };
}

// ---- Changing --------------------------------------------------------------------------------------------

function mapPane(node: Node, id: string, change: (pane: Pane) => Node): Node {
  if (node.type === "pane") return node.id === id ? change(node) : node;
  return { ...node, a: mapPane(node.a, id, change), b: mapPane(node.b, id, change) };
}

/** A node without a pane: the sibling takes the split's place. Absent when the pane was the whole tree. */
function without(node: Node, id: string): Node | undefined {
  if (node.type === "pane") return node.id === id ? undefined : node;
  const a = without(node.a, id);
  const b = without(node.b, id);
  if (a === undefined) return b;
  if (b === undefined) return a;
  return { ...node, a, b };
}

/** Where the focus goes when its pane is gone: the first pane of what is left. */
function refocus(arrangement: Arrangement): Arrangement {
  return paneById(arrangement, arrangement.focus) === undefined ? { ...arrangement, focus: panesOf(arrangement.root)[0]!.id } : arrangement;
}

export function focusPane(arrangement: Arrangement, paneId: string): Arrangement {
  return paneById(arrangement, paneId) === undefined || arrangement.focus === paneId ? arrangement : { ...arrangement, focus: paneId };
}

export function activate(arrangement: Arrangement, tabId: string): Arrangement {
  const pane = paneOfTab(arrangement, tabId);
  if (pane === undefined) return arrangement;
  return { ...arrangement, focus: pane.id, root: mapPane(arrangement.root, pane.id, (one) => ({ ...(one as Pane), active: tabId })) };
}

/** A label no other tab has: the same host or shell a second time is numbered. */
function distinct(arrangement: Arrangement, label: string | undefined): string | undefined {
  if (label === undefined) return undefined;
  const taken = new Set(panesOf(arrangement.root).flatMap((one) => one.tabs.map((tab) => tab.label)));
  let at = 1;
  while (taken.has(at === 1 ? label : `${label} ${at}`)) at += 1;
  return at === 1 ? label : `${label} ${at}`;
}

const tabOf = (id: string, kind: TabKind, label: string | undefined, page?: string): Tab => ({ id, kind, ...(label === undefined ? {} : { label }), ...(page === undefined ? {} : { page }) });

/** A new tab at the end of a pane, in front. */
export function addTab(arrangement: Arrangement, paneId: string, kind: TabKind, label?: string, page?: string): Arrangement {
  if (paneById(arrangement, paneId) === undefined) return arrangement;
  const [id, next] = mint(arrangement, "t");
  const made = tabOf(id, kind, distinct(arrangement, label), page);
  return { ...next, focus: paneId, root: mapPane(next.root, paneId, (one) => ({ ...(one as Pane), tabs: [...(one as Pane).tabs, made], active: id })) };
}

/** Which tab comes to the front when this one leaves a pane: the one after it, else the one before. */
function afterLeaving(pane: Pane, tabId: string): string | undefined {
  const at = pane.tabs.findIndex((tab) => tab.id === tabId);
  const rest = pane.tabs.filter((tab) => tab.id !== tabId);
  return pane.active !== tabId ? pane.active : (rest[at] ?? rest[at - 1])?.id;
}

/** The tab leaves its pane; a pane left with none is removed, and the last pane of all stays, empty. */
function detach(arrangement: Arrangement, tabId: string): Arrangement {
  const pane = paneOfTab(arrangement, tabId);
  if (pane === undefined) return arrangement;
  const rest = pane.tabs.filter((tab) => tab.id !== tabId);
  if (rest.length > 0) return { ...arrangement, root: mapPane(arrangement.root, pane.id, () => ({ ...pane, tabs: rest, active: afterLeaving(pane, tabId) })) };
  const root = without(arrangement.root, pane.id);
  if (root === undefined) return { ...arrangement, root: { ...pane, tabs: [], active: undefined } };
  return refocus({ ...arrangement, root });
}

export function closeTab(arrangement: Arrangement, tabId: string): Arrangement {
  return detach(arrangement, tabId);
}

/** The pane goes with its tabs, empty or not; the last pane of all stays. */
export function closePane(arrangement: Arrangement, paneId: string): Arrangement {
  if (paneById(arrangement, paneId) === undefined) return arrangement;
  const root = without(arrangement.root, paneId);
  return root === undefined ? arrangement : refocus({ ...arrangement, root });
}

/** The pane splits, and an empty pane takes the new side: to its right, or below it. Focus goes to the new pane. */
export function splitPane(arrangement: Arrangement, paneId: string, into: "right" | "down"): Arrangement {
  const [splitId, afterSplit] = mint(arrangement, "s");
  const [fresh, next] = mint(afterSplit, "p");
  if (paneById(next, paneId) === undefined) return arrangement;
  const empty: Pane = { type: "pane", id: fresh, tabs: [], active: undefined };
  const root = mapPane(next.root, paneId, (one) => ({ type: "split", id: splitId, dir: into === "right" ? "row" : "column", ratio: 0.5, a: one, b: empty }));
  return { ...next, root, focus: fresh };
}

/** The tab goes into a pane, at an index or at the end. */
export function moveTabToPane(arrangement: Arrangement, tabId: string, paneId: string, index?: number): Arrangement {
  const from = paneOfTab(arrangement, tabId);
  const tab = tabById(arrangement, tabId);
  if (from === undefined || tab === undefined || paneById(arrangement, paneId) === undefined) return arrangement;
  // Dropped back where it is: only its place among the pane's tabs can change.
  if (from.id === paneId) {
    const others = from.tabs.filter((one) => one.id !== tabId);
    const at = Math.min(Math.max(index ?? others.length, 0), others.length);
    const tabs = [...others.slice(0, at), tab, ...others.slice(at)];
    return { ...arrangement, focus: paneId, root: mapPane(arrangement.root, paneId, () => ({ ...from, tabs, active: tabId })) };
  }
  const left = detach(arrangement, tabId);
  const target = paneById(left, paneId)!;
  const at = Math.min(Math.max(index ?? target.tabs.length, 0), target.tabs.length);
  const tabs = [...target.tabs.slice(0, at), tab, ...target.tabs.slice(at)];
  return { ...left, focus: paneId, root: mapPane(left.root, paneId, () => ({ ...target, tabs, active: tabId })) };
}

/** The tab goes to a new pane on one side of a pane. A pane's only tab dropped on its own edge stays as it is. */
export function moveTabToEdge(arrangement: Arrangement, tabId: string, paneId: string, side: Side): Arrangement {
  const from = paneOfTab(arrangement, tabId);
  const tab = tabById(arrangement, tabId);
  if (from === undefined || tab === undefined || paneById(arrangement, paneId) === undefined) return arrangement;
  if (from.id === paneId && from.tabs.length === 1) return arrangement;
  const left = detach(arrangement, tabId);
  const [splitId, afterSplit] = mint(left, "s");
  const [fresh, next] = mint(afterSplit, "p");
  const made: Pane = { type: "pane", id: fresh, tabs: [tab], active: tab.id };
  const first = side === "left" || side === "top";
  const root = mapPane(next.root, paneId, (one) => ({ type: "split", id: splitId, dir: side === "left" || side === "right" ? "row" : "column", ratio: 0.5, a: first ? made : one, b: first ? one : made }));
  return { ...next, root, focus: fresh };
}

export function resizeSplit(arrangement: Arrangement, splitId: string, ratio: number): Arrangement {
  const put = (node: Node): Node => (node.type === "pane" ? node : node.id === splitId ? { ...node, ratio: clampRatio(ratio) } : { ...node, a: put(node.a), b: put(node.b) });
  return { ...arrangement, root: put(arrangement.root) };
}

/** The next or the previous tab of a pane comes to the front, wrapping round. */
export function cycleTab(arrangement: Arrangement, paneId: string, by: 1 | -1): Arrangement {
  const pane = paneById(arrangement, paneId);
  if (pane === undefined || pane.tabs.length < 2) return arrangement;
  const at = pane.tabs.findIndex((tab) => tab.id === pane.active);
  return activate(arrangement, pane.tabs[(at + by + pane.tabs.length) % pane.tabs.length]!.id);
}

/** A tab a Job opens itself joins the pane that holds that kind; with none, it opens as a split on the right of everything. */
export function arrive(arrangement: Arrangement, kind: TabKind, label?: string, page?: string): Arrangement {
  const holder = holderOf(arrangement, kind);
  if (holder !== undefined) return addTab(arrangement, holder.id, kind, label, page);
  const [splitId, afterSplit] = mint(arrangement, "s");
  const [fresh, afterPane] = mint(afterSplit, "p");
  const [tabId, next] = mint(afterPane, "t");
  const made: Pane = { type: "pane", id: fresh, tabs: [tabOf(tabId, kind, distinct(arrangement, label), page)], active: tabId };
  return { ...next, root: { type: "split", id: splitId, dir: "row", ratio: 0.62, a: arrangement.root, b: made } };
}

/** The same arrangement, as a copy no other holds. */
export const copyOf = (arrangement: Arrangement): Arrangement => JSON.parse(JSON.stringify(arrangement)) as Arrangement;

