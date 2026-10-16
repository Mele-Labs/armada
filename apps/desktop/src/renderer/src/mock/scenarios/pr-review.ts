// The review surface over the flight deck's ship: five pull requests in two repositories, three that teammates asked his
// review of (one with threads already) and two that agents of his opened (one held by a Session). Fleet serves none of it
// yet, so the seed is the mock's (`draft.prReview`, `../pr-review-pages.ts`) and every act is recorded and goes no further.
// The walk `prReviewTab` plays it: the surface on a tab of its own.

import type { Scenario } from "../moment";
import { reviewSeed } from "../pr-review-pages";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

export const s262PrReview: Scenario = {
  ...ship,
  name: "pr-review",
  says: "The flight deck with pull requests to review: three asked of him in two repositories, two that his agents opened",
  draft: { ...ship.draft, prReview: reviewSeed() },
};
