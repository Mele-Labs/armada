// Kit's keyboard: the tiles move like the cockpit's glass. Bare arrows go to the nearest tile in
// that direction, across both glasses; `move_focus` steps through them in the order they are drawn;
// `open_focused` and `open` step into a tile's first control, and `close` steps back out to the
// tile. In a text field, `close` and Down on an empty one hand the keys back to the tiles.
//
// **The cursor is DOM focus**, as `list-keyboard.ts` has it: a tile reached by the mouse, by Tab
// or by a key is the same tile, and the one remembered for a field to hand back to is read off
// focus rather than kept beside it.

import { useEffect, useRef, type RefObject } from "react";

import { isPressed, pressedSlot } from "@armada/components";
import { useListKeydown } from "@armada/screens/src/list-keyboard";
import { holdsText } from "@armada/screens/src/keys";

const TILE = "[data-kit-tile]";
const CONTROL = 'button, select, input, textarea, a[href], [tabindex]:not([tabindex="-1"])';
const ARROWS = ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"] as const;
type Arrow = (typeof ARROWS)[number];

/**
 * The tile nearest `from` in the arrow's direction, or `undefined` at the glass's edge. Distance is
 * edge to edge along the arrow, with the gap across it weighed double, so a tile straight ahead
 * beats a nearer one off to the side and a tile spanning the glass is straight ahead of every one.
 */
export function toward(from: DOMRect, arrow: Arrow, tiles: HTMLElement[], self?: Element): HTMLElement | undefined {
  const gap = (a0: number, a1: number, b0: number, b1: number): number => Math.max(0, b0 - a1, a0 - b1);
  let best: HTMLElement | undefined;
  let score = Infinity;
  for (const tile of tiles) {
    if (tile === self) continue;
    const to = tile.getBoundingClientRect();
    const along =
      arrow === "ArrowDown" ? to.top - from.bottom
      : arrow === "ArrowUp" ? from.top - to.bottom
      : arrow === "ArrowRight" ? to.left - from.right
      : from.left - to.right;
    // Overlapping by a border's width still counts as beyond it.
    if (along < -1) continue;
    const across =
      arrow === "ArrowDown" || arrow === "ArrowUp"
        ? gap(from.left, from.right, to.left, to.right)
        : gap(from.top, from.bottom, to.top, to.bottom);
    const at = Math.max(0, along) + across * 2;
    if (at < score) {
      score = at;
      best = tile;
    }
  }
  return best;
}

/** Kit's keys, answered while focus is inside `root` or nowhere at all. */
export function useKitKeys(root: RefObject<HTMLElement | null>): void {
  // The tile a field hands the keys back to: the one last focused.
  const last = useRef<HTMLElement | null>(null);
  useEffect(() => {
    const element = root.current;
    if (element === null) return;
    const noted = (event: FocusEvent): void => {
      const tile = (event.target as HTMLElement).closest<HTMLElement>(TILE);
      if (tile !== null) last.current = tile;
    };
    element.addEventListener("focusin", noted);
    return () => element.removeEventListener("focusin", noted);
  }, [root]);

  useListKeydown((event) => {
    const element = root.current;
    if (element === null || event.defaultPrevented) return;
    const target = event.target as HTMLElement;
    const nowhere = target === document.body || target === document.documentElement;
    if (!nowhere && !element.contains(target)) return;

    const tiles = Array.from(element.querySelectorAll<HTMLElement>(TILE));
    if (tiles.length === 0) return;
    const tile = target.closest<HTMLElement>(TILE);
    const claim = (to: HTMLElement | null | undefined): void => {
      if (to === null || to === undefined) return;
      event.preventDefault();
      to.focus();
    };
    // Every act's keys are the keymap's. The arrows that move across the glass are spatial rather
    // than an act, and stay bare keys — the cockpit's own exception.
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    const arrow = bare && (ARROWS as readonly string[]).includes(event.key) ? (event.key as Arrow) : undefined;

    if (holdsText(target)) {
      // A select answers its own arrows; `close` takes it back to its tile.
      if (target instanceof HTMLSelectElement) return isPressed("close", event) ? claim(tile) : undefined;
      // A field: `close`, or Down with nothing typed, hands the keys back to the tile last held —
      // or, before any was, the one below the field.
      const empty = (target as HTMLInputElement).value === "";
      if (isPressed("close", event) || (arrow === "ArrowDown" && empty)) {
        const back = last.current?.isConnected ? last.current : toward(target.getBoundingClientRect(), "ArrowDown", tiles);
        claim(back ?? tiles[0]);
      }
      return;
    }

    // A control inside a tile: `close` steps back out to the tile. Enter is the control's own.
    if (tile !== null && target !== tile && isPressed("close", event)) return claim(tile);

    if (arrow !== undefined) {
      const from = tile ?? (nowhere ? null : target);
      return claim(from === null ? tiles[0] : toward(from.getBoundingClientRect(), arrow, tiles, from));
    }

    // j and k: through the tiles in the order they are drawn, clamped at either end.
    const step = pressedSlot("move_focus", event);
    if (step === 0 || step === 1) {
      const at = tile === null ? -1 : tiles.indexOf(tile);
      const to = at < 0 ? 0 : Math.min(tiles.length - 1, Math.max(0, at + (step === 0 ? 1 : -1)));
      return claim(tiles[to]);
    }

    // Enter, or `o`, on a tile: into its first control. A tile that is a reading alone has none, and
    // the press is left to fall through.
    if (target === tile && (isPressed("open_focused", event) || isPressed("open", event))) {
      return claim(tile.querySelector<HTMLElement>(CONTROL));
    }
  });
}
