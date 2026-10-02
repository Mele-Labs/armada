// The Land board on Job 2 as a real Fleet served it: finished, its pull
// request merged. The owner's twelve notes of 1 Oct 2026, answered;
// `land-real-job.test.tsx` holds the claims.

import { button, role, text, walk } from "../walk";

export const aFinishedJobReadPlainly = walk("recorded/landed-and-merged", [
  { look: role("link", "#1750"), say: "The pull request's state is a badge beside it, as the Job's is" },
  { look: text(/^Landed$/), say: "Where the work got to: no count, and no rule for completing on a Job that completed" },
  { look: text("no verdict recorded"), say: "The criterion says what answered it, and nothing sums it up" },
  { look: button("Dispatch", { exact: true }), say: "The act is the title row's Dispatch, with its key: a new Job" },
  { look: text(/^Merged$/), say: "The same badge on the pull request's row" },
  { look: text(/^The run$/i), say: "The run's table spans its region" },
  { look: text(/validation-that\.jsonl$/), say: "Every value wraps rather than being cut off" },
]);
