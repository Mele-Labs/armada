// Take A of the review surface: a PR Review mode on the Cockpit's Flight control. Pressing it swaps the panels into
// review (the list on the left, the pull request as an explanation in the stage, the review tools in the Flight
// control below it), and pressing it again puts the Cockpit back exactly as it was. Over the `pr-review` scenario; the other take is
// `prReviewTab`, the same parts on a tab of their own.

import { theExplanation, theNewTool, theSessionsOwn, theTools, toTheList } from "../pr-review-steps";
import { button, inside, region, walk } from "../walk";

const controls = region("Flight controls");
const work = region("Jobs and Sessions");

const prReviewMode = walk("pr-review", [
  { look: work, say: "The Cockpit on its work" },
  { hover: inside(controls, button("PR Review")), say: "PR Review is a mode in the Flight control, and ⇧P is its key" },
  { key: "Shift+P", on: work, say: "The Cockpit's panels swap into review" },
  ...toTheList,
  ...theExplanation,
  ...theTools,
  ...theSessionsOwn,
  ...theNewTool,
  { key: "Shift+P", on: region("Pull requests"), say: "⇧P again, and the Cockpit is back" },
  { look: work, say: "The list of Jobs and Sessions, as it was left" },
]);

export { prReviewMode as "pr-review-mode" };
