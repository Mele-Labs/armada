// Moving a row of the Work list: which rows can be moved, where ⌥↑ and ⌥↓ would put one, and the drag that does the same with a pointer.
// A row moves among the rows of its own repository heading and never across one; a Session handed a plan group stands under its parent
// and goes where the parent goes.

import { useRef, useState } from "react";
import type { PointerEvent } from "react";

import { bandsOf, type Banding } from "./bands";
import type { Subject } from "./ship";

/** A live Session that stands in the list on its own account: not an ended one, nor a pull request, nor one the list hangs under its parent. */
export function movableIn(rows: readonly Subject[]): ReadonlySet<string> {
  return new Set(rows.filter((one) => one.kind === "session" && one.past === undefined && one.linked?.depth === undefined).map((one) => one.key));
}

/** The rows that share a heading, in the order drawn: all of them where the list has no headings. */
const groupsOf = (rows: readonly Subject[], banding: readonly Banding[] | undefined): Subject[][] => (banding === undefined ? [[...rows]] : bandsOf(rows, banding).map((band) => band.subjects));

/** The heading a row is under, or the empty key where there are none. */
export const bandOf = (rows: readonly Subject[], banding: readonly Banding[] | undefined, key: string): string =>
  banding === undefined ? "" : (bandsOf(rows, banding).find((band) => band.subjects.some((one) => one.key === key))?.key ?? "");

/** The row ⌥↑ or ⌥↓ passes `key` over: the next movable row above or below in its heading, if there is one. */
export function neighbourOf(rows: readonly Subject[], banding: readonly Banding[] | undefined, key: string, by: 1 | -1): string | undefined {
  const movable = movableIn(rows);
  if (!movable.has(key)) return undefined;
  const group = groupsOf(rows, banding).find((one) => one.some((row) => row.key === key));
  const same = group?.filter((one) => movable.has(one.key)) ?? [];
  return same[same.findIndex((one) => one.key === key) + by]?.key;
}

/** Where a drag would drop: before or after the row it is over. */
export type Drop = { key: string; after: boolean };
export type Dragging = { key: string; drop: Drop | undefined };

/** Pointer travel, in pixels, before a press on a row is a drag and not a click. */
const SLOP = 4;

const rowKeyOf = (item: HTMLElement): string | undefined => item.querySelector<HTMLElement>("[data-row]")?.dataset.row;

/**
 * A row picked up with the pointer and put down among the rows of its heading. **Rows are measured, not hit-tested**, so the drop is
 * wherever the pointer's height falls between their middles; the row's own click is swallowed once, so putting it down does not open it.
 * `among` picks the list items a row moves among, each holding a `[data-row]`: the Work list's by default, the Projects tab's cards there.
 */
export function useReorder(onMove: (key: string, target: string, after: boolean) => void, among = ".deck-list__item[data-movable]") {
  const [dragging, setDragging] = useState<Dragging>();
  const swallow = useRef(false);

  const grab = (event: PointerEvent<HTMLElement>, key: string, band: string): void => {
    if (event.button !== 0 || !event.isPrimary) return;
    const from = event.clientY;
    let live = false;
    let drop: Drop | undefined;
    const peers = () => [...document.querySelectorAll<HTMLElement>(among)].filter((item) => item.dataset.movable === band && rowKeyOf(item) !== key);
    const aim = (y: number): Drop | undefined => {
      const items = peers();
      const below = items.find((item) => {
        const box = item.getBoundingClientRect();
        return y < box.top + box.height / 2;
      });
      const target = below ?? items.at(-1);
      const at = target === undefined ? undefined : rowKeyOf(target);
      return at === undefined ? undefined : { key: at, after: below === undefined };
    };
    const move = (next: globalThis.PointerEvent) => {
      if (!live && Math.abs(next.clientY - from) < SLOP) return;
      live = true;
      drop = aim(next.clientY);
      setDragging({ key, drop });
    };
    const done = (next: globalThis.PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", done);
      window.removeEventListener("pointercancel", done);
      setDragging(undefined);
      if (!live) return;
      // The click that follows the release is not a press on the row; one that never comes must not leave the next real one swallowed.
      swallow.current = true;
      window.setTimeout(() => (swallow.current = false), 80);
      if (next.type === "pointerup" && drop !== undefined) onMove(key, drop.key, drop.after);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", done);
    window.addEventListener("pointercancel", done);
  };

  /** On the row's capture phase: true once, for the click a drag ends in. */
  const swallowed = (): boolean => {
    const was = swallow.current;
    swallow.current = false;
    return was;
  };

  return { dragging, grab, swallowed };
}
