// A Job waiting on the owner reads `Needs review`, not `Awaiting review`: the
// badge in its header, and the same word on its step cards. The owner's request
// of 4 Oct 2026, on his Job 3 where a Judge question was waiting for him. Job 2
// just before it landed is the recorded Job at `awaiting_review`.

import { tab, text, walk } from "../walk";

export const needsReviewInTheHeader = walk("real/job-2-at-review", [
  { look: text("Needs review"), say: "The header's badge, for a Job at awaiting_review" },
  { press: tab("Workflow"), say: "The run, top to bottom" },
  { look: text("needs review"), say: "The step waiting on you reads the same word" },
]);
