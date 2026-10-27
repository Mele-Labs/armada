// The reader's Files tab: every changed file's diff, one under the other, as the pull request panel draws them. Over the `pr-review` scenario.

import { button, inside, place, region, tab, walk } from "../walk";

const list = region("Pull requests");
const reader = region("Pull request #1899");

const prReviewFiles = walk("pr-review", [
  { press: place("Reviews"), say: "Reviews" },
  { press: inside(list, button(/#1899/)), say: "Open one a teammate asked for" },
  { press: inside(reader, tab("Files")), say: "Files, beside Overview, Checks and Commits" },
  { look: inside(reader, region("crates/fleet/src/plan_usage.rs")), say: "Every changed file's diff, one under the other" },
  { press: inside(reader, tab("Overview")), say: "Back to Overview, where the explanations are" },
]);

export { prReviewFiles as "pr-review-files" };
