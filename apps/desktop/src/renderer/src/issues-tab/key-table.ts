// The Issues tab's keys, **read off the registry** (`issues_*` rows of `actions.toml`, scope `issues`), so Settings → Keyboard
// moves a key and the caps, the tooltips and the handlers in `keys.tsx` follow. Nothing here spells a key.

import { formatSlot, parseChord, slotsOf, type Chord } from "@armada/components";

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
};

export { capOf } from "../pr-review/key-table";

/**
 * The two acts on a reported bug. **Not the registry's yet**: the keys wait on the look, and move to `issues_keep` and
 * `issues_drop` rows (scope `issues`) with the wiring. Start a Job on a report is `issues_start`, as on an issue. `D` is
 * also `issues_describe`: it drops where a report holds focus, in the list or the reader, and describes everywhere else.
 */
export const REPORT_KEYS = { keep: "K", drop: "D" } as const;
export const REPORT_CHORDS: { keep: Chord; drop: Chord } = { keep: parseChord(REPORT_KEYS.keep)!, drop: parseChord(REPORT_KEYS.drop)! };
