// The widths of the deck's two side columns, which a handle on each inner edge drags. Kept in this browser as dragged
// (`localStorage`) and clamped on each look to the window as it is now, so a column dragged
// wide in a big window comes back to what it was when the window does. A column never dragged has no entry and is drawn at
// its resting width. All lengths are multiples of `--space-12`, the unit `deck.css` sizes the columns by.

import { useCallback, useLayoutEffect, useMemo, useState, type RefObject } from "react";
import { clampToRange, tokenPx } from "@armada/components";

export type Side = "left" | "right";
type Kept = Partial<Record<Side, number>>;

const KEY = "armada.bridge.deck-columns";

/** Resting widths, as `deck.css` draws the grid before anything is dragged, in `--space-12`. */
const REST: Record<Side, number> = { left: 6, right: 5.75 };
/** The narrowest each column goes before its rows clip: the left's row is a kind, a title, a few words and a mark. */
const FLOOR: Record<Side, number> = { left: 4.5, right: 4.75 };
/** The least the stage keeps: a Session's thread. */
const CENTRE_FLOOR = 10;
/** Layouts drawn with no right column, so the left may take what the right would have held. `bare` is the Cockpit a layout emptied the right column of. */
const NO_RIGHT: ReadonlySet<string> = new Set(["issues", "projects", "reviews", "bare", "session"]);

export type Columns = {
  left: number;
  right: number;
  /** Each column's range given the other as drawn, so the centre keeps its floor. */
  range: Record<Side, { min: number; max: number }>;
};

/** What each column is drawn at, given the room the columns share (the grid less its gaps) and the unit in px. */
export function settle(kept: Kept, room: number, unit: number, layout = "", leftBonus = 0, rightMin = 0): Columns {
  // A view open in the right column that needs room (a Job's Lanes) lifts its floor for as long as it is open, over a width dragged to, and gives it back when it shuts.
  const floor = { left: FLOOR.left * unit, right: Math.max(FLOOR.right * unit, rightMin) };
  const centre = CENTRE_FLOOR * unit;
  const wanted = (side: Side) => kept[side] ?? REST[side] * unit + (side === "left" ? leftBonus : 0);
  const rightMax = Math.max(floor.right, room - floor.left - centre);
  const right = NO_RIGHT.has(layout) ? 0 : clampToRange(wanted("right"), { min: floor.right, max: rightMax });
  const leftMax = Math.max(floor.left, room - right - centre);
  const left = clampToRange(wanted("left"), { min: floor.left, max: leftMax });
  return { left, right, range: { left: { min: floor.left, max: leftMax }, right: { min: floor.right, max: Math.max(floor.right, room - left - centre) } } };
}

const keyFor = (layout: string) => (layout === "" ? KEY : `${KEY}.${layout}`);

function read(layout: string): Kept {
  try {
    const parsed: unknown = JSON.parse(window.localStorage.getItem(keyFor(layout)) ?? "{}");
    if (typeof parsed !== "object" || parsed === null) return {};
    const { left, right } = parsed as Record<string, unknown>;
    return { ...(typeof left === "number" && Number.isFinite(left) ? { left } : {}), ...(typeof right === "number" && Number.isFinite(right) ? { right } : {}) };
  } catch {
    return {};
  }
}

function write(layout: string, next: Kept): void {
  try {
    window.localStorage.setItem(keyFor(layout), JSON.stringify(next));
  } catch {
    // A failed write leaves the widths unremembered past this window.
  }
}

/**
 * The columns' widths for the grid in `grid`, and how to move or reset one. `undefined` until the grid has been measured,
 * when the stylesheet's own resting widths hold and no handle is drawn.
 */
export function useColumns(grid: RefObject<HTMLElement | null>, layout = "", leftBonus = 0, rightMin = 0) {
  // Kept for each layout (nothing selected, a Session open), each read from the store the first time it is drawn.
  const [all, setAll] = useState<Record<string, Kept>>({});
  const kept = all[layout] ?? read(layout);
  const [room, setRoom] = useState<number | undefined>(undefined);
  useLayoutEffect(() => {
    const element = grid.current;
    if (element === null) return;
    const gaps = NO_RIGHT.has(layout) ? 1 : 2;
    const measure = () => setRoom(element.clientWidth - gaps * tokenPx("--space-3"));
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const watching = new ResizeObserver(measure);
    watching.observe(element);
    return () => watching.disconnect();
  }, [grid, layout]);
  const columns = useMemo(() => {
    const unit = tokenPx("--space-12");
    return room === undefined || !Number.isFinite(room) || !Number.isFinite(unit) ? undefined : settle(kept, room, unit, layout, leftBonus, rightMin);
  }, [kept, room, layout, leftBonus, rightMin]);
  const resize = useCallback((side: Side, width: number) => {
    setAll((was) => {
      const next = { ...(was[layout] ?? read(layout)), [side]: Math.round(width) };
      write(layout, next);
      return { ...was, [layout]: next };
    });
  }, [layout]);
  const reset = useCallback((side: Side) => {
    setAll((was) => {
      const { [side]: _gone, ...next } = was[layout] ?? read(layout);
      write(layout, next);
      return { ...was, [layout]: next };
    });
  }, [layout]);
  return { columns, resize, reset };
}
