// Pull requests stacked on one another. Completed lists a stack parent first, one lane through it, the root's base as a tag, and a chip on a child whose parent is
// not the row above, which finds the parent's row. Over the `stacked-pull-requests` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const landing = region("Completed", { exact: true });

const stackedPullRequests = walk("stacked-pull-requests", [
  { press: deckRow("Split the export into steps"), say: "Six pull requests, one merged and five stacked on one another" },
  { look: inside(landing, role("list", "Pull requests")), say: "The stack reads parent first under one lane. The root says what it is based on, and the second child of the root names it in a chip" },
  { press: inside(landing, role("link", "Stacked on pull request #3102")), say: "The chip finds the parent's row" },
]);

export { stackedPullRequests as "stacked-pull-requests" };
