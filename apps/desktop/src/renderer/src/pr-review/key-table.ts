// The review surface's keys, **read off the registry** (`review_*` rows of `actions.toml`, scope `review`), so Settings →
// Keyboard moves a key and the caps, the tooltips and the handlers in `keys.tsx` follow. Nothing here spells a key.

import { formatSlot, slotsOf } from "@armada/components";

const capOfSlot = (id: string, at: number): string => {
  const slot = slotsOf(id)[at];
  return slot === undefined ? "" : formatSlot(slot);
};

/** The review surface's own keys as this person has them now. `⇧` letters are capitals here, as the keymap's grammar has them. */
export const REVIEW_KEYS = {
  get list() {
    return capOfSlot("review_panel", 0);
  },
  get reader() {
    return capOfSlot("review_panel", 1);
  },
  get tools() {
    return capOfSlot("review_panel", 2);
  },
  get lists() {
    return capOfSlot("review_lists", 0);
  },
  get filter() {
    return capOfSlot("review_filter", 0);
  },
  get search() {
    return capOfSlot("review_search", 0);
  },
  get quick() {
    return capOfSlot("review_quick", 0);
  },
  get ask() {
    return capOfSlot("review_ask", 0);
  },
  get description() {
    return capOfSlot("review_description", 0);
  },
  get where() {
    return capOfSlot("review_where", 0);
  },
};

/** A key as it is drawn on a cap: a capital is Shift and the letter. */
export const capOf = (key: string): string => (/^[A-Z]$/.test(key) ? `⇧${key}` : key);
