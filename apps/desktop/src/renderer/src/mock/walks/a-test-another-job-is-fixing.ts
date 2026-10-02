// A Check failed on a test another Job is already fixing: Overview's lead
// names the fix and links to it, the failed Check's row on the Record names
// it too, and the fix Job's own lead counts the Jobs waiting on it. The
// owner's decision of 2 Oct 2026, #1673; `job-detail.test.tsx` holds the claims.

import { button, tab, text, walk } from "../walk";

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
  { look: text(/2 Jobs wait on it/), say: "The fix's own lead counts the Jobs waiting on it, never the list" },
]);
