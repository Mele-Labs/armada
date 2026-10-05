// The same, as a Fleet before 23.5 served it: no title and no comment count.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { job2AtReviewBefore235 } from "../job-2-at-review";

export const s030JobTwoAtReviewBefore235: Scenario = holding("real/job-2-at-review-before-23-5", job2AtReviewBefore235().name, [job2AtReviewBefore235()], {
  opens: job2AtReviewBefore235().job.id,
});
