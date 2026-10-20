// The pages the deck links to because no tab or System entry carries them. A page with none of these and no tab or entry is
// reached from the palette, `⌘K`, by name, as Checks is. `reachable.test.ts` reads this to prove no page is reachable only by a
// place the window no longer draws. #2301.

import { SURFACE } from "@armada/shell";

export const DECK_LINK = {
  /** The Bays band. */
  bays: SURFACE.worktrees,
  /** The merge queue's mark. */
  queue: SURFACE.mergeLine,
  /** The control beside the repository picker. */
  picker: SURFACE.manifest,
} as const;
