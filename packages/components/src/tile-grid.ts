// The keyboard of a surface drawn as the cockpit's glass: tiles on a grid, one Tab stop.
//
// **A grid is one Tab stop.** Of the tiles at a level only one is tabbable, the one last
// focused (or the one marked current, or the first), and a control inside a tile is tabbable only
// while its tile is. So Tab enters the grid, steps through the tile it landed on, and leaves; the
// arrows and `j`/`k` choose the tile. A tile inside a tile (a section's entries) is a level of its
// own, roved the same way inside its parent.
//
// **The cursor is DOM focus**, `list-keyboard.ts`'s rule: a tile reached by the mouse, by Tab or by
// a key is the same tile. Bare arrows are spatial rather than an act and stay bare keys, the
// cockpit's own exception; every other key is the keymap's.

import { useEffect, useRef, type RefObject } from "react";

import { isPressed, pressedSlot } from "./keymap";

export type TileGridOptions = {
  /** What a tile is, as a selector. Tiles may nest. */
  tile: string;
  /** What inside a tile takes focus for it, as a selector. Absent is the tile itself, which then carries its own `tabIndex`. */
  cursor?: string;
  /** What Enter on the cursor, and `o` anywhere in a tile, do. Absent: a cursor that is a button is pressed; any other goes into the tile. Return `false` to leave the press alone. */
  onOpen?: (tile: HTMLElement) => boolean | void;
  /** Whether `j` and `k` step through the tiles. `false` where the surface has given them another meaning. */
  steps?: boolean;
};

type Arrow = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";
const ARROWS: readonly string[] = ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];
const FOCUSABLE = 'a[href], button, input, select, textarea, summary, [tabindex], [contenteditable="true"]';
/** Set on what this file took out of the Tab order, holding the `tabindex` it had, so it can be given back. */
const ROVED = "data-roved";

const fieldOf = (target: EventTarget | null): HTMLElement | null =>
  target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName)) ? target : null;

/**
 * The tile nearest `from` the way an arrow points, by the boxes as drawn: the gap along the arrow,
 * edge to edge, with the gap across it weighed double, so a tile straight ahead beats a nearer one
 * off to the side and Down stays in its column while the column has a tile below.
 */
export function tileToward(from: HTMLElement, tiles: readonly HTMLElement[], arrow: Arrow): HTMLElement | undefined {
  const a = from.getBoundingClientRect();
  const gap = (a0: number, a1: number, b0: number, b1: number) => Math.max(0, b0 - a1, a0 - b1);
  let best: HTMLElement | undefined;
  let score = Infinity;
  for (const tile of tiles) {
    if (tile === from) continue;
    const b = tile.getBoundingClientRect();
    const along = arrow === "ArrowDown" ? b.top - a.bottom : arrow === "ArrowUp" ? a.top - b.bottom : arrow === "ArrowRight" ? b.left - a.right : a.left - b.right;
    // Overlapping by a border's width still counts as beyond it.
    if (along < -1) continue;
    const across = arrow === "ArrowDown" || arrow === "ArrowUp" ? gap(a.left, a.right, b.left, b.right) : gap(a.top, a.bottom, b.top, b.bottom);
    const at = Math.max(0, along) + across * 2;
    if (at < score) [best, score] = [tile, at];
  }
  return best;
}

/** Roves `root`'s tiles: one Tab stop a level, and the keys that move between them. */
export function useTileGrid(root: RefObject<HTMLElement | null>, options: TileGridOptions): void {
  const latest = useRef(options);
  latest.current = options;
  // The tile last focused at each level, keyed by the tile it sits in (`null` at the top).
  const current = useRef(new Map<Element | null, Element>());

  // The element the listeners are on. A root drawn after the first render (a surface that first says
  // nothing is served) is picked up on the render that draws it, so the check runs after every one.
  const attached = useRef<{ element: HTMLElement; detach: () => void } | null>(null);
  // Forgotten as well as detached, so a remount (StrictMode runs every effect twice) attaches again.
  useEffect(
    () => () => {
      attached.current?.detach();
      attached.current = null;
    },
    [],
  );
  useEffect(() => {
    const element = root.current;
    if (attached.current?.element === element) return;
    attached.current?.detach();
    attached.current = element === null ? null : { element, detach: attach(element) };
  });

  function attach(element: HTMLElement): () => void {
    const { tile: TILE } = latest.current;
    const parentOf = (tile: Element): Element | null => tile.parentElement?.closest(TILE) ?? null;
    const cursorOf = (tile: Element): HTMLElement | null => {
      const { cursor } = latest.current;
      return cursor === undefined ? (tile as HTMLElement) : tile.querySelector<HTMLElement>(cursor);
    };
    const drawn = (one: Element) => one.getClientRects().length > 0;
    const peersOf = (level: Element | null): HTMLElement[] =>
      [...element.querySelectorAll<HTMLElement>(TILE)].filter((one) => parentOf(one) === level && drawn(one));
    const currentOf = (level: Element | null): Element | undefined => {
      const peers = peersOf(level);
      const kept = current.current.get(level);
      if (kept !== undefined && peers.includes(kept as HTMLElement)) return kept;
      return peers.find((one) => one.matches('[aria-current="true"], [aria-selected="true"]') || cursorOf(one)?.matches('[aria-current="true"], [aria-selected="true"]')) ?? peers[0];
    };

    // Tabbable only where every tile it sits in is the current one at its level.
    const rove = () => {
      for (const one of element.querySelectorAll<HTMLElement>(FOCUSABLE)) {
        // A `tabindex="-1"` this file did not set is the author's: never in the Tab order.
        if (one.getAttribute("tabindex") === "-1" && !one.hasAttribute(ROVED)) continue;
        let open = true;
        for (let tile = one.closest(TILE); tile !== null && open; tile = parentOf(tile)) open = currentOf(parentOf(tile)) === tile;
        if (open && one.hasAttribute(ROVED)) {
          const was = one.getAttribute(ROVED)!;
          if (was === "") one.removeAttribute("tabindex");
          else one.setAttribute("tabindex", was);
          one.removeAttribute(ROVED);
        } else if (!open && (!one.hasAttribute(ROVED) || one.getAttribute("tabindex") !== "-1")) {
          // Also where something wrote it back since (a tooltip's trigger re-takes its stop on every render): that value is the one to give back.
          one.setAttribute(ROVED, one.getAttribute("tabindex") ?? "");
          one.setAttribute("tabindex", "-1");
        }
      }
    };

    const noted = (event: FocusEvent) => {
      const target = event.target as HTMLElement;
      for (let tile = target.closest(TILE); tile !== null; tile = parentOf(tile)) current.current.set(parentOf(tile), tile);
      rove();
    };

    const press = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const { onOpen, steps = true } = latest.current;
      const target = event.target as HTMLElement;
      const claim = (to: HTMLElement | null | undefined) => {
        if (to === null || to === undefined) return;
        event.preventDefault();
        to.focus();
      };
      const bare = !(event.metaKey || event.ctrlKey || event.altKey || event.shiftKey);
      const arrow = bare && ARROWS.includes(event.key) ? (event.key as Arrow) : undefined;
      const tile = target.closest<HTMLElement>(TILE);
      const level = tile === null ? null : parentOf(tile);
      const top = () => {
        const one = currentOf(null);
        return one === undefined ? null : cursorOf(one);
      };

      // An editor keeps every key it is given.
      if (target.closest("[data-keeps-keys]") !== null) return;
      const field = fieldOf(target);
      if (field !== null) {
        // A select answers its own arrows; Escape takes it back to its tile.
        if (field.tagName === "SELECT") return isPressed("close", event) ? claim(tile === null ? null : cursorOf(tile)) : undefined;
        // Escape, whatever the field holds (a draft is kept), or Down from an empty one, hands the keys to the tiles.
        const empty = (field as HTMLInputElement).value === "";
        if (isPressed("close", event) || (arrow === "ArrowDown" && empty)) claim(tile === null ? top() : cursorOf(tile));
        return;
      }
      if (tile === null) return;
      const cursor = cursorOf(tile);
      const onCursor = target === cursor;

      if (arrow !== undefined) return claim(cursorOf(tileToward(tile, peersOf(level), arrow) ?? tile));
      const step = pressedSlot("move_focus", event);
      if (steps && (step === 0 || step === 1)) {
        const peers = peersOf(level);
        const at = peers.indexOf(tile);
        return claim(cursorOf(peers[Math.min(peers.length - 1, Math.max(0, at + (step === 0 ? 1 : -1)))]!));
      }
      // Escape on a control in a tile comes out to the tile; on a tile inside another, out to that one.
      if (isPressed("close", event)) {
        if (!onCursor) return claim(cursor);
        return level === null ? undefined : claim(cursorOf(level));
      }
      // Enter belongs to whatever holds focus first: a link or a button answers it itself.
      const native = target instanceof HTMLButtonElement || target instanceof HTMLAnchorElement;
      if ((onCursor && !native && isPressed("open_focused", event)) || isPressed("open", event)) {
        if (onOpen !== undefined) {
          if (onOpen(tile) !== false) event.preventDefault();
          return;
        }
        if (cursor instanceof HTMLButtonElement) return event.preventDefault(), cursor.click();
        // Into the tile: its first tile a level down, or its first control.
        const inner = peersOf(tile)[0];
        rove();
        const next = inner === undefined ? [...tile.querySelectorAll<HTMLElement>(FOCUSABLE)].find((one) => one !== cursor && drawn(one) && one.getAttribute("tabindex") !== "-1") : cursorOf(inner);
        return claim(next);
      }
    };

    // From nowhere, the arrows and `j`/`k` land on the grid's tile, as the cockpit's glass does.
    const idle = (event: KeyboardEvent) => {
      if (event.defaultPrevented || (event.target !== document.body && event.target !== document.documentElement)) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"]') !== null) return;
      const bare = !(event.metaKey || event.ctrlKey || event.altKey || event.shiftKey);
      const step = latest.current.steps === false ? -1 : pressedSlot("move_focus", event);
      if (!(bare && ARROWS.includes(event.key)) && step !== 0 && step !== 1) return;
      const one = currentOf(null);
      const to = one === undefined ? null : cursorOf(one);
      if (to === null) return;
      event.preventDefault();
      to.focus();
    };

    rove();
    const watch = new MutationObserver(rove);
    watch.observe(element, { childList: true, subtree: true, attributes: true, attributeFilter: ["tabindex"] });
    element.addEventListener("focusin", noted);
    element.addEventListener("keydown", press);
    window.addEventListener("keydown", idle);
    return () => {
      watch.disconnect();
      element.removeEventListener("focusin", noted);
      element.removeEventListener("keydown", press);
      window.removeEventListener("keydown", idle);
    };
  }
}
