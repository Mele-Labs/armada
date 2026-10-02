// The review gate's record, as the owner picked it on 2 Oct 2026 (#1680): a card
// per section straight under the lead, never folded, on his Job 2 just before
// it landed.

import { inside, region, role, text, walk } from "../walk";

export const theRecordAtTheGate = walk("real/job-2-at-review", [
  { look: region("What you asked for"), say: "The record, open under the lead: a card per section" },
  {
    look: role("link", "Pull request #1750"),
    say: "The pull request, small: its title, branch, the Job's Checks and its comments. The card opens it",
  },
  { look: region("The work"), say: "The figures under it, read and not editable" },
  {
    look: inside(region("What proves it"), text("Plan the change")),
    say: "What proves it: every step's Checks, not only the step waiting on you",
  },
  { look: region("What was not checked"), say: "What was not checked, without Fleet's standing preamble" },
  { look: region("What was done"), say: "What was done" },
  { look: region("What was skipped"), say: "What was skipped" },
]);
