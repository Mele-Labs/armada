// The same, with no title kept but one the live read of the pull request holds.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { job2AtReviewLiveTitle } from "@armada/jobs/fake";

export const s040JobTwoAtReviewLiveTitle: Scenario = holding("real/job-2-at-review-live-title", job2AtReviewLiveTitle().name, [job2AtReviewLiveTitle()], {
  opens: job2AtReviewLiveTitle().job.id,
});
