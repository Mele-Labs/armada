// The Reviews tab's Fleet: the four lists, a pull request's page, its patches, a posted review and a resolved
// thread, served from the scenario's mock forge (`draft.prReview`, `../review-forge.ts`). A scenario with none
// answers that nothing is behind the window, as every other surface's slice does.

import type { ReviewsApi } from "../../../../shared/api/reviews";
import type { Slice } from "../fake-context";
import { NO_FORGE, createMockForge } from "../review-forge";

export const reviews: Slice<ReviewsApi, Record<string, never>> = {
  name: "reviews",
  state: {},
  api: (scenario) => (scenario.draft?.prReview === undefined ? NO_FORGE : createMockForge(scenario.draft.prReview)),
};
