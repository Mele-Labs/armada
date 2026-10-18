// On All repositories with nothing open the Cockpit's merge line is a stack of one row per repository, each led by its own
// main light: one row shows, it scrolls across, ↑ and ↓ move the stack, and a red main brings its row into view. A Session or a
// pull request open on the stage narrows it to that one's repository, and one picked draws its own.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const queue = region("Merge queue");

const mergeLineByRepository = walk("merge-line-by-repository", [
  { look: queue, say: "On All with nothing open the merge line is a stack, one row per repository, one row at a time" },
  { look: inside(queue, button(/^Main is red in storefront/)), say: "storefront's main is red, so its row comes into view first, led by its own main" },
  { look: inside(queue, button(/^#3011/)), say: "storefront's pull requests across the row" },
  { key: "ArrowUp", on: inside(queue, button(/^#3011/)), say: "↑ moves the stack to the row above. A wheel turned over the line does the same, and it snaps" },
  { look: inside(queue, button(/^#2101/)), say: "armada's row, led by its main. Left alone, the stack moves to the next repository every 15 seconds" },
  { press: inside(list, button("Draft the migration notes")), say: "Open a Job in armada" },
  { look: inside(queue, button(/^#2101/)), say: "The line is armada's alone, one row, its light named main" },
  { press: inside(list, button("Lay out the invoice PDF")), say: "Open a Job in storefront" },
  { look: inside(queue, button(/^#3011/)), say: "The line is storefront's now" },
  { press: button("Repository"), say: "Pick one repository" },
  { press: role("menuitem", "armada"), say: "armada" },
  { look: inside(queue, button(/^#2102/)), say: "One picked draws that repository's line, whatever is open" },
]);

export { mergeLineByRepository as "merge-line-by-repository" };
