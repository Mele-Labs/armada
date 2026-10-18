// The review surface: a Reviews tab in the title bar's places, with the list, the reader and the review tools in the
// Cockpit's own grid. Over the `pr-review` scenario.

import { theExplanation, theNewTool, theSessionsOwn, theTools, toTheList } from "../pr-review-steps";
import { place, region, walk } from "../walk";

const prReviewTab = walk("pr-review", [
  { look: place("Reviews"), say: "Reviews is a place of its own, beside Retros" },
  { press: place("Reviews"), say: "The pull requests waiting on him" },
  ...toTheList,
  ...theExplanation,
  ...theTools,
  ...theSessionsOwn,
  ...theNewTool,
  { press: place("Cockpit"), say: "The Cockpit is untouched, on its work" },
  { look: region("Jobs", { exact: true }), say: "Nothing of the review is in it" },
]);

export { prReviewTab as "pr-review-tab" };
