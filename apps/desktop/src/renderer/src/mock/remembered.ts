// The window a browser test opens, and a walk opens too: one whose person has
// met every guide and remembers nothing about how it was last read. Set up
// before each test by `guides-met.ts`, which carries the reasoning, and before
// a walk by `main.tsx`, so a walk the owner plays starts where its test did.

import { GUIDES } from "@armada/components";

/** What `guidance.tsx` writes. */
const GUIDES_KEY = "armada.bridge.guides";

/** Everything a window remembers about how it was last read. `guides-met.ts` says why a test forgets it. */
const REMEMBERED = [
  "armada.bridge.dock-width",
  "armada.bridge.guide-list-width",
  "armada.bridge.left-collapsed",
  "armada.bridge.left-width",
  "armada.bridge.lessons-tab",
  "armada.bridge.panels-open",
  "armada.bridge.plan-view",
  "armada.bridge.sheet-width",
  "armada.bridge.workflow-view",
];

/** Forget how the window was last read: its widths, its arrangements, what was open. */
export function forgetHowItWasRead(): void {
  for (const key of REMEMBERED) window.localStorage.removeItem(key);
}

/** Every guide already met, so no card opens by itself over the press being made. */
export function meetEveryGuide(): void {
  window.localStorage.setItem(
    GUIDES_KEY,
    JSON.stringify({ off: false, cardSeen: true, met: GUIDES.map((guide) => guide.piece) }),
  );
}
