import type { Guide } from "./guide";

/**
 * What the Overview's Merge line panel is, and what its marks say. The panel
 * reports the line and the reason each waiting branch is not in the running
 * turn; the reading of those marks is here.
 *
 * **No `docs/concepts/` page holds the merge line**: `landing.md` covers a
 * Job's members, not `armada land`'s queue.
 */
export const GUIDE_MERGE_LINE: Guide = {
  number: 22,
  group: "run",
  title: "What is the merge line?",
  piece: "merge-line.what",
  steps: [
    "Merges onto main take turns, and each branch's Checks run against the main it will land on.",
    "The number beside a branch is the order it merges in, the running batch first.",
    "A bracketed group is gated together.",
    "Branch mark: left out of this turn because it clashes with another branch in it.",
    "Ban mark: left out of this turn because it clashes with main.",
    "Arrow mark: kept its place after a red, and is next up.",
    "Commit mark: joined after the turn began.",
    "No mark: queued behind.",
  ],
};
