// The Manifest surface's keys: its tiles move like the cockpit's glass.
//
// **The cursor is DOM focus**, `list-keyboard.ts`'s rule, so a tile reached by
// Tab, by the mouse or by an arrow is the same cursor. A tile is any element
// carrying `data-tile` inside the viewscreen: a Check or Command on the run
// view, a section or a declared entry on the form. Tiles nest — an entry
// inside its section — and the keys move between the tiles of one level:
// Enter goes in a level, Esc comes back out.
//
// **Bare arrows are spatial, not an act**, the cockpit's own exception; every
// other key is read through the keymap. The file view is an editor and keeps
// every key it is given.

import type { RefObject } from "react";

import { isPressed, pressedSlot } from "@armada/components";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

const TILE = "[data-tile]";
const ARROWS = ["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"] as const;
type Arrow = (typeof ARROWS)[number];
const FOCUSABLE =
  'button:not(:disabled), input:not(:disabled), select:not(:disabled), textarea:not(:disabled), [tabindex]:not([tabindex="-1"])';

/** The tile a tile sits inside, or `null` for one at the top. */
function levelOf(tile: Element): Element | null {
  return tile.parentElement?.closest(TILE) ?? null;
}

/** Every tile drawn at the same level as `of`, in document order. */
function peersOf(root: HTMLElement, of: Element | null): HTMLElement[] {
  const level = of === null ? null : levelOf(of);
  return Array.from(root.querySelectorAll<HTMLElement>(TILE)).filter(
    (tile) => tile.getClientRects().length > 0 && levelOf(tile) === level,
  );
}

/**
 * The nearest tile in the arrow's direction, by the centres: the distance along
 * the arrow plus twice the drift across it, so Down in a grid stays in its column.
 */
function nearest(from: HTMLElement, peers: HTMLElement[], arrow: Arrow): HTMLElement | undefined {
  const at = from.getBoundingClientRect();
  const x = at.left + at.width / 2;
  const y = at.top + at.height / 2;
  let best: HTMLElement | undefined;
  let score = Infinity;
  for (const peer of peers) {
    if (peer === from) continue;
    const box = peer.getBoundingClientRect();
    const dx = box.left + box.width / 2 - x;
    const dy = box.top + box.height / 2 - y;
    const [along, across] =
      arrow === "ArrowRight" ? [dx, dy] : arrow === "ArrowLeft" ? [-dx, dy] : arrow === "ArrowDown" ? [dy, dx] : [-dy, dx];
    if (along <= 1) continue;
    const here = along + 2 * Math.abs(across);
    if (here < score) {
      score = here;
      best = peer;
    }
  }
  return best;
}

/** Into a tile: its first tile a level down, or its first control where it has none. */
function enter(tile: HTMLElement): boolean {
  const inner = Array.from(tile.querySelectorAll<HTMLElement>(TILE)).find((one) => levelOf(one) === tile);
  const next = inner ?? Array.from(tile.querySelectorAll<HTMLElement>(FOCUSABLE)).find((one) => one.getClientRects().length > 0);
  next?.focus();
  return next !== undefined;
}

export function useManifestKeys(screen: RefObject<HTMLElement | null>, { editor }: { editor: boolean }): void {
  useListKeydown((event) => {
    const root = screen.current;
    if (root === null || event.defaultPrevented) return;
    const target = event.target instanceof HTMLElement ? event.target : null;
    // Nothing focused is the body: the tiles take the keys from there, as the cockpit's glass does.
    // From a control elsewhere — the rail, the picker — only `j` and `k` reach in; its arrows are its own.
    const idle = target === null || target === document.body;
    const outside = !idle && !root.contains(target);
    if (outside && (holdsText(target) || pressedSlot("move_focus", event) > 1 || pressedSlot("move_focus", event) === -1)) return;
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    const arrow = bare && (ARROWS as readonly string[]).includes(event.key) ? (event.key as Arrow) : null;
    const claim = () => event.preventDefault();

    if (holdsText(target)) {
      if (editor) return;
      // Esc leaves a field for its tile, whatever it holds — the draft is kept either way. Down only
      // from an empty one, where it cannot be moving a caret or stepping a number.
      const empty = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement ? target.value === "" : false;
      if (isPressed("close", event) || (arrow === "ArrowDown" && empty)) {
        const home = target?.closest<HTMLElement>(TILE) ?? peersOf(root, null)[0];
        if (home !== undefined) {
          claim();
          home.focus();
        }
      }
      return;
    }

    const tile = target?.closest<HTMLElement>(TILE) ?? null;
    const onTile = tile !== null && tile === target;
    const step = pressedSlot("move_focus", event);

    // From nowhere, or from a control that is not a tile, the first press lands on the tiles: the
    // one picked if it is drawn, the first otherwise.
    if (!onTile && (step === 0 || step === 1 || (idle && arrow !== null))) {
      const top = peersOf(root, tile);
      const landing = top.find((one) => one.getAttribute("aria-current") === "true") ?? top[0];
      if (landing === undefined) return;
      claim();
      landing.focus();
      return;
    }
    if (!onTile || tile === null) return;

    if (arrow !== null) {
      claim();
      nearest(tile, peersOf(root, tile), arrow)?.focus();
      return;
    }
    if (step === 0 || step === 1) {
      const peers = peersOf(root, tile);
      const at = peers.indexOf(tile);
      claim();
      peers[Math.min(peers.length - 1, Math.max(0, at + (step === 0 ? 1 : -1)))]?.focus();
      return;
    }
    // A tile that is a button answers Enter itself; `o` is the same press, named.
    if (tile instanceof HTMLButtonElement) {
      if (isPressed("open", event)) {
        claim();
        tile.click();
      }
      return;
    }
    if (isPressed("open_focused", event) || isPressed("open", event)) {
      if (enter(tile)) claim();
      return;
    }
    if (isPressed("close", event)) {
      const up = levelOf(tile);
      if (up instanceof HTMLElement) {
        claim();
        up.focus();
      }
    }
  });
}
