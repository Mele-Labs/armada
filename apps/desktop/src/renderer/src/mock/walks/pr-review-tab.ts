// Take B of the review surface: a Reviews tab in the title bar's places, with the same three parts as the Cockpit's
// review mode, so the Cockpit stays on the work. Over the `pr-review` scenario; the other take is `prReviewMode`.

import { theReview, theSessionsOwn, toTheList } from "../pr-review-steps";
import { place, region, walk } from "../walk";

const prReviewTab = walk("pr-review", [
  { look: place("Reviews"), say: "Reviews is a place of its own, beside Retros" },
  { press: place("Reviews"), say: "The pull requests waiting on him" },
  ...toTheList,
  ...theReview,
  ...theSessionsOwn,
  { press: place("Cockpit"), say: "The Cockpit is untouched, on its work" },
  { look: region("Jobs and Sessions"), say: "Nothing of the review is in it" },
]);

export { prReviewTab as "pr-review-tab" };
