/**
 * Where a frame is drawn round what it holds, and which frame a node put down
 * lands in — a Studio's Zones and Clusters, `#1620`.
 *
 * **Positions inside a frame are measured from its corner**, which is React
 * Flow's `parentId` and Fleet's `within` alike: dragging a frame moves what it
 * holds with no write to any of them. A frame's size is never kept. It is
 * worked out here, every render, from where what it holds sits.
 */

type At = { x: number; y: number };

/** Enough of a node on the board to frame it. */
export type Framing = {
  id: string;
  kind: string;
  parentId?: string;
  position: At;
  measured?: { width?: number; height?: number };
};

/** Room between a frame's edge and what it holds, and above it for its head. Fleet lays out to these. */
export const FRAME_INSET = 24;
export const FRAME_HEAD = 48;
/** A frame holding nothing: wide enough for one card, and its head. */
const EMPTY = { width: 288, height: 160 };
/** A card not measured yet: the card's own width, and its usual height. */
const UNMEASURED = { width: 240, height: 140 };

/** Whether a node of this kind draws as a frame. */
export function frames(kind: string): boolean {
  return kind === "zone" || kind === "cluster";
}

/**
 * Whether a frame of `frame`'s kind takes a node of `kind` put down in it. A
 * Zone takes anything but a Zone. **A Cluster takes nothing by a drop**: its
 * Notes are its record, made by grouping, so a Note is never dropped into one.
 */
function takes(frame: string, kind: string): boolean {
  return frame === "zone" && kind !== "zone";
}

function size(node: Framing, sizes: ReadonlyMap<string, { width: number; height: number }>) {
  const framed = sizes.get(node.id);
  if (framed !== undefined) return framed;
  return {
    width: node.measured?.width ?? UNMEASURED.width,
    height: node.measured?.height ?? UNMEASURED.height,
  };
}

/** Enough of a node to say what holds it. */
type Held = { id: string; parentId?: string };

/** How many frames a node is inside. */
function depth(node: Held, byId: ReadonlyMap<string, Held>): number {
  let held = 0;
  let at = node.parentId;
  while (at !== undefined && held < 8) {
    held += 1;
    at = byId.get(at)?.parentId;
  }
  return held;
}

/** Every node, parents before what they hold — which React Flow requires. */
export function parentsFirst<N extends Held>(nodes: readonly N[]): N[] {
  const byId = new Map<string, Held>(nodes.map((node) => [node.id, node]));
  return nodes
    .map((node, at) => ({ node, at, depth: depth(node, byId) }))
    .sort((a, b) => a.depth - b.depth || a.at - b.at)
    .map(({ node }) => node);
}

/**
 * Each frame's size: round what it holds, out to its far edges and an inset
 * past them — innermost first, so a Zone holding a Cluster is sized round the
 * Cluster's own frame. `leaving` is held out of every frame, so a node being
 * put down is measured against the frame as it was without it.
 */
export function frameSizes(
  nodes: readonly Framing[],
  leaving?: string,
): Map<string, { width: number; height: number }> {
  const byId = new Map<string, Held>(nodes.map((node) => [node.id, node]));
  const sizes = new Map<string, { width: number; height: number }>();
  const framesInnermostFirst = nodes
    .filter((node) => frames(node.kind))
    .sort((a, b) => depth(b, byId) - depth(a, byId));
  for (const frame of framesInnermostFirst) {
    let width = EMPTY.width;
    let height = EMPTY.height;
    for (const held of nodes) {
      if (held.parentId !== frame.id || held.id === leaving) continue;
      const box = size(held, sizes);
      width = Math.max(width, held.position.x + box.width + FRAME_INSET);
      height = Math.max(height, held.position.y + box.height + FRAME_INSET);
    }
    sizes.set(frame.id, { width, height });
  }
  return sizes;
}

/** Where a node sits on the board itself: its position added to every frame's it is inside. */
export function onTheBoard(id: string, byId: ReadonlyMap<string, Framing>): At {
  const node = byId.get(id);
  if (node === undefined) return { x: 0, y: 0 };
  if (node.parentId === undefined) return node.position;
  const corner = onTheBoard(node.parentId, byId);
  return { x: node.position.x + corner.x, y: node.position.y + corner.y };
}

/** Where a node put down sits now: the frame it is in, and its spot from that frame's corner. */
export type Landing = { within: string | null; position: At };

/**
 * Which frame a node put down at `position` lands in, and where in it.
 *
 * **A Note in a Cluster stays in it**, wherever it is dropped, and is kept
 * inside its corner. Anything else lands in the Zone its middle is over, or on
 * the board where that is no Zone — so dragging a node out of a Zone takes it
 * out, and onto one puts it in. A Zone always lands on the board.
 */
export function landing(nodes: readonly Framing[], id: string, position: At): Landing {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const moving = byId.get(id);
  if (moving === undefined) return { within: null, position };
  const parent = moving.parentId === undefined ? undefined : byId.get(moving.parentId);
  if (parent?.kind === "cluster") {
    return { within: parent.id, position: insideCorner(position) };
  }
  const sizes = frameSizes(nodes, id);
  const corner = parent === undefined ? { x: 0, y: 0 } : onTheBoard(parent.id, byId);
  const box = size(moving, sizes);
  const middle = {
    x: corner.x + position.x + box.width / 2,
    y: corner.y + position.y + box.height / 2,
  };
  const over = nodes.find((frame) => {
    if (frame.id === id || !takes(frame.kind, moving.kind)) return false;
    const at = onTheBoard(frame.id, byId);
    const framed = sizes.get(frame.id) ?? EMPTY;
    return (
      middle.x >= at.x && middle.x <= at.x + framed.width && middle.y >= at.y && middle.y <= at.y + framed.height
    );
  });
  const board = { x: corner.x + position.x, y: corner.y + position.y };
  if (over === undefined) return { within: null, position: board };
  const into = onTheBoard(over.id, byId);
  return { within: over.id, position: insideCorner({ x: board.x - into.x, y: board.y - into.y }) };
}

/**
 * Where a node of `kind` put down by a press at `at` on the board lands — the
 * owner, 2 Oct 2026: *"Pressing inside a Zone places the armed kind there and
 * puts it in that Zone."*
 *
 * **The frame a drop would take it into**, so the two ways in agree: a Zone
 * takes anything but a Zone, and a Cluster takes nothing, so a press inside a
 * Cluster lands in the Zone round it, or on the board. Where frames overlap,
 * the one drawn last is the one pressed.
 */
export function pressedIn(nodes: readonly Framing[], kind: string, at: At): Landing {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const sizes = frameSizes(nodes);
  const over = [...nodes].reverse().find((frame) => {
    if (!takes(frame.kind, kind)) return false;
    const corner = onTheBoard(frame.id, byId);
    const framed = sizes.get(frame.id) ?? EMPTY;
    return at.x >= corner.x && at.x <= corner.x + framed.width && at.y >= corner.y && at.y <= corner.y + framed.height;
  });
  if (over === undefined) return { within: null, position: at };
  const into = onTheBoard(over.id, byId);
  return { within: over.id, position: insideCorner({ x: at.x - into.x, y: at.y - into.y }) };
}

/** A spot in a frame, kept clear of its edge and its head. */
function insideCorner(position: At): At {
  return { x: Math.max(FRAME_INSET, position.x), y: Math.max(FRAME_HEAD, position.y) };
}
