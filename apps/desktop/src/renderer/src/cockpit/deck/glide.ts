// The battlefield coming online and moving without jumps. Powering on, the field holds empty until Fleet answers and its inputs have been
// quiet a moment, then the territories come in one at a time, each growing from its middle (`useFill`). After that a layout that changes
// glides there from where it was drawn (`useGlide`) instead of being drawn at once in its new place. Reduced motion draws each at once.

import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";

import type { Power } from "../boot";
import { reducedMotion } from "./exit";
import type { Layout, Placed, Point, Rect, Territory, Tile } from "./layout";

/** How long the inputs must be still after Fleet answers before the first territory comes in. */
export const FILL_QUIET_MS = 600;
/** The longest the field waits for quiet after Fleet answers: a Fleet that streams changes without a pause still fills. */
export const FILL_MOST_MS = 2_500;
/** How far apart the territories come in. */
export const FILL_STEP_MS = 220;
/** How long a territory takes to glide to its new place: longer than a row's travel, since a map moves further. */
export const GLIDE_MS = 700;
/** The size a territory grows from, against its own. */
const SEED = 0.2;

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const lerpRect = (a: Rect, b: Rect, t: number): Rect => ({ x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t), w: lerp(a.w, b.w, t), h: lerp(a.h, b.h, t) });
const lerpPoint = <P extends Point>(a: Point, b: P, t: number): P => ({ ...b, x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) });
const ease = (t: number) => 1 - (1 - t) ** 4;

/** A territory shrunk about its middle by `s`, and everything on it: what a territory grows from. */
function shrunk(one: Territory, s: number): Territory {
  const c = { x: one.rect.x + one.rect.w / 2, y: one.rect.y + one.rect.h / 2 };
  const point = <P extends Point>(p: P): P => ({ ...p, x: c.x + (p.x - c.x) * s, y: c.y + (p.y - c.y) * s });
  const rect = (r: Rect): Rect => ({ x: c.x + (r.x - c.x) * s, y: c.y + (r.y - c.y) * s, w: r.w * s, h: r.h * s });
  return {
    ...one,
    rect: rect(one.rect),
    ...(one.pad === undefined ? {} : { pad: rect(one.pad) }),
    groups: one.groups.map((g) => ({ ...g, rect: rect(g.rect) })),
    tiles: one.tiles.map((tile) => ({ ...tile, rect: rect(tile.rect), cx: c.x + (tile.cx - c.x) * s, cy: c.y + (tile.cy - c.y) * s })),
    base: { ...point(one.base), w: one.base.w * s },
    pads: one.pads.map(point),
    goal: one.goal === undefined ? undefined : point(one.goal),
  };
}

/** A tile on its way from `a` to `b`. Its pills are `b`'s, laid into the rect it has now. */
function tileBetween(a: Tile | undefined, b: Tile, t: number): Tile {
  if (a === undefined) return b;
  const rect = lerpRect(a.rect, b.rect, t);
  const sx = b.rect.w === 0 ? 1 : rect.w / b.rect.w;
  const sy = b.rect.h === 0 ? 1 : rect.h / b.rect.h;
  const pills = b.pills === undefined ? undefined : { ...b.pills, x: rect.x + (b.pills.x - b.rect.x) * sx, y: rect.y + (b.pills.y - b.rect.y) * sy, cw: b.pills.cw * sx, ch: b.pills.ch * sy, pw: b.pills.pw * sx, ph: b.pills.ph * sy };
  return { ...b, rect, cx: lerp(a.cx, b.cx, t), cy: lerp(a.cy, b.cy, t), ...(pills === undefined ? {} : { pills }) };
}

function territoryBetween(a: Territory, b: Territory, t: number): Territory {
  const groups = new Map(a.groups.map((g) => [g.dir, g] as const));
  const tiles = new Map(a.tiles.map((tile) => [tile.path, tile] as const));
  const pads = new Map(a.pads.map((pad) => [pad.key, pad] as const));
  return {
    ...b,
    rect: lerpRect(a.rect, b.rect, t),
    ...(b.pad === undefined ? {} : { pad: a.pad === undefined ? b.pad : lerpRect(a.pad, b.pad, t) }),
    groups: b.groups.map((g) => {
      const was = groups.get(g.dir);
      return was === undefined ? g : { ...g, rect: lerpRect(was.rect, g.rect, t) };
    }),
    tiles: b.tiles.map((tile) => tileBetween(tiles.get(tile.path), tile, t)),
    base: { ...lerpPoint(a.base, b.base, t), w: lerp(a.base.w, b.base.w, t) },
    pads: b.pads.map((pad) => {
      const was = pads.get(pad.key);
      return was === undefined ? pad : lerpPoint(was, pad, t);
    }),
    goal: b.goal === undefined ? undefined : a.goal === undefined ? b.goal : lerpPoint(a.goal, b.goal, t),
  };
}

/** The layout `t` of the way from `a` to `b`: `b`'s parts, each where it is on its way. A territory `a` lacks grows from its middle. */
export function layoutBetween(a: Layout, b: Layout, t: number): Layout {
  const was = new Map(a.territories.map((one) => [one.root, one] as const));
  const territories = b.territories.map((one) => territoryBetween(was.get(one.root) ?? shrunk(one, SEED), one, t));
  const tiles = new Map(territories.flatMap((one) => one.tiles.map((tile) => [tile.path, tile] as const)));
  const grown = new Map(b.territories.filter((one) => !was.has(one.root)).map((one) => [one.root, shrunk(one, SEED)] as const));
  const at = new Map<string, Placed>();
  for (const [key, to] of b.at) {
    const from = a.at.get(key);
    const seed = grown.get(to.root);
    const start = from ?? (seed === undefined ? to : { ...to, x: seed.rect.x + seed.rect.w / 2, y: seed.rect.y + seed.rect.h / 2 });
    at.set(key, { ...to, x: lerp(start.x, to.x, t), y: lerp(start.y, to.y, t), s: lerp(start.s, to.s, t) });
  }
  const bars = new Map([...b.bars].map(([path, bar]) => {
    const from = a.bars.get(path);
    const tile = tiles.get(bar.tile.path) ?? bar.tile;
    return [path, from === undefined ? { ...bar, tile } : { ...bar, tile, rect: lerpRect(from.rect, bar.rect, t), cx: lerp(from.cx, bar.cx, t), cy: lerp(from.cy, bar.cy, t) }] as const;
  }));
  const over = new Map([...b.over].map(([key, list]) => [key, list.map((one) => ({ ...one, tile: tiles.get(one.tile.path) ?? one.tile }))] as const));
  return { territories, at, bars, over, clashes: b.clashes };
}

const near = (a: number, b: number) => Math.abs(a - b) < 0.5;
const sameRect = (a: Rect, b: Rect) => near(a.x, b.x) && near(a.y, b.y) && near(a.w, b.w) && near(a.h, b.h);

/** Whether two layouts put everything in the same place, so there is nothing to glide. */
export function sameGeometry(a: Layout, b: Layout): boolean {
  if (a.territories.length !== b.territories.length || a.at.size !== b.at.size) return false;
  for (let i = 0; i < a.territories.length; i += 1) {
    const x = a.territories[i]!;
    const y = b.territories[i]!;
    if (x.root !== y.root || !sameRect(x.rect, y.rect) || x.tiles.length !== y.tiles.length || x.pads.length !== y.pads.length) return false;
  }
  for (const [key, one] of a.at) {
    const other = b.at.get(key);
    if (other === undefined || !near(one.x, other.x) || !near(one.y, other.y)) return false;
  }
  return true;
}

/** The layout with only the territories named, and what stands on them. */
export function clipLayout(layout: Layout, roots: ReadonlySet<string>): Layout {
  const territories = layout.territories.filter((one) => roots.has(one.root));
  const tiles = new Set(territories.flatMap((one) => one.tiles));
  const at = new Map([...layout.at].filter(([, one]) => roots.has(one.root)));
  return {
    territories,
    at,
    bars: new Map([...layout.bars].filter(([, bar]) => tiles.has(bar.tile))),
    over: new Map([...layout.over].filter(([key]) => at.has(key))),
    clashes: layout.clashes.filter((one) => roots.has(one.root)),
  };
}

/**
 * The territories drawn while the field powers on, in `order`; none is every one. Held empty on standby, and after Fleet answers until
 * `inputs` has not changed for a moment; then one more each step. `on` is false for a map that does not power on, which draws all.
 */
export function useFill(order: readonly string[], inputs: unknown, power: Power, on: boolean): ReadonlySet<string> | undefined {
  const [count, setCount] = useState<number | undefined>(on && power !== "live" && power !== "fault" ? 0 : undefined);
  // When Fleet answered: inputs that never go quiet still start the fill once the longest wait is out.
  const answered = useRef<number | null>(null);
  if (power === "standby") answered.current = null;
  else if (answered.current === null) answered.current = Date.now();
  // A Fleet that went away and is coming back: the field holds again.
  useEffect(() => {
    if (on && power === "standby") setCount(0);
  }, [on, power]);
  useEffect(() => {
    if (count === undefined || power === "standby") return;
    if (count === 0) {
      const wait = Math.min(FILL_QUIET_MS, Math.max(0, (answered.current ?? Date.now()) + FILL_MOST_MS - Date.now()));
      const timer = window.setTimeout(() => setCount(reducedMotion() ? undefined : 1), wait);
      return () => window.clearTimeout(timer);
    }
    if (count >= order.length) {
      setCount(undefined);
      return;
    }
    const timer = window.setTimeout(() => setCount(count + 1), FILL_STEP_MS);
    return () => window.clearTimeout(timer);
    // While none is drawn, each change of the inputs starts the quiet again.
  }, [count, power, order.length, count === 0 ? inputs : undefined]);
  const key = count === undefined ? undefined : order.slice(0, count).join("\n");
  return useMemo(() => (key === undefined ? undefined : new Set(key === "" ? [] : key.split("\n"))), [key]);
}

/**
 * The layout as drawn: `target`, reached by gliding from what was drawn before. Snaps where `snap` changes (the pane took a new size),
 * where nothing moved, where `on` is false, and under reduced motion.
 */
export function useGlide(target: Layout, snap: string, on: boolean): Layout {
  const [shown, setShown] = useState(target);
  const drawn = useRef(target);
  const size = useRef(snap);
  const frame = useRef(0);
  useLayoutEffect(() => {
    const from = drawn.current;
    const resized = size.current !== snap;
    size.current = snap;
    if (!on || resized || from === target || reducedMotion() || sameGeometry(from, target)) {
      drawn.current = target;
      setShown(target);
      return;
    }
    const start = performance.now();
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / GLIDE_MS);
      const next = t >= 1 ? target : layoutBetween(from, target, ease(t));
      drawn.current = next;
      setShown(next);
      frame.current = t >= 1 ? 0 : window.requestAnimationFrame(step);
    };
    frame.current = window.requestAnimationFrame(step);
    return () => window.cancelAnimationFrame(frame.current);
  }, [target, snap, on]);
  return shown;
}
