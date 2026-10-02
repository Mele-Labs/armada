// A Check failed on a test another Job is already fixing: Overview's lead
// names the fix and links to it, the failed Check's row on the Record names
// it too, and the fix Job's own lead lists the Jobs waiting on it, each a
// link. The owner's decisions of 2 Oct 2026, #1673; `job-detail.test.tsx`
// holds the claims.

import { button, inside, role, tab, text, walk } from "../walk";

const parked = role("list", "Waiting on this fix");

const fix = button("Fix the selectors test broken on main", { exact: true });

export const aTestAnotherJobIsFixing = walk("breakage/fixed-elsewhere", [
  { look: text("cargo_nextest failed"), say: "A Check failed on this Job" },
  {
    look: text(/is already fixing this, and this Job is kept off the test's files/),
    say: "Another Job is already fixing the test, and Fleet keeps this Job off its files",
  },
  { press: tab("Record"), say: "The Job's Record, newest first" },
  {
    look: text("Failed — Fix the selectors test broken on main is already fixing it"),
    say: "The failed Check's row names the fix too",
  },
  { press: tab("Overview"), say: "Back to the lead" },
  { press: fix, say: "The fix's name opens that Job" },
  { look: parked, say: "The fix's own lead lists the Jobs waiting on it" },
  { press: inside(parked, button("Memoise the manifest list", { exact: true })), say: "Each one opens its Job" },
  { look: text("Memoise the manifest list"), say: "The Job parked on the fix" },
]);
