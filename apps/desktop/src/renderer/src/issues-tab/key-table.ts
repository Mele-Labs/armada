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
  get describe() {
    return capOfSlot("issues_describe", 0);
  },
  get start() {
    return capOfSlot("issues_start", 0);
  },
};

export { capOf } from "../pr-review/key-table";
