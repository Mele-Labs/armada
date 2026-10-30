// The Issues tab's keys, **read off the registry** (`issues_*` rows of `actions.toml`, scope `issues`), so Settings → Keyboard
// moves a key and the caps, the tooltips and the handlers in `keys.tsx` follow. Nothing here spells a key.

import { formatSlot, slotsOf } from "@armada/components";

const capOfSlot = (id: string, at: number): string => {
  const slot = slotsOf(id)[at];
  return slot === undefined ? "" : formatSlot(slot);
};

export const ISSUES_KEYS = {
  get list() {
    return capOfSlot("issues_panel", 0);
  },
  get reader() {
    return capOfSlot("issues_panel", 1);
  },
  get queries() {
    return capOfSlot("issues_queries", 0);
  },
  get search() {
    return capOfSlot("issues_search", 0);
  },
  get describe() {
    return capOfSlot("issues_describe", 0);
  },
  get start() {
    return capOfSlot("issues_start", 0);
  },
  /** The two acts on a reported bug, `issues_keep` and `issues_drop` (scope `report`). `D` is also `issues_describe`, which a report holding focus does not get. */
  get keep() {
    return capOfSlot("issues_keep", 0);
  },
  get drop() {
    return capOfSlot("issues_drop", 0);
  },
};

export { capOf } from "../pr-review/key-table";
