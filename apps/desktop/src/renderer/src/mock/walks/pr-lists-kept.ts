// The Reviews tab on a relaunch: the last reading is drawn at once and replaced when Fleet answers. Over the `pr-review-kept` scenario.

import { button, inside, place, region, walk } from "../walk";

const list = region("Pull requests");
const refresh = inside(list, button("Refresh"));

const prListsKept = walk("pr-review-kept", [
  { press: place("Reviews"), say: "Reviews, after a relaunch" },
  { look: inside(list, button(/#1899/)), say: "The rows are there before Fleet has answered" },
  { hover: refresh, say: "Fetching, behind the rows already drawn" },
  { look: inside(list, button(/#212/)), say: "The fresh reading has one more pull request" },
  { hover: refresh, say: "Fetched <1m ago" },
]);

export { prListsKept as "pr-lists-kept" };
