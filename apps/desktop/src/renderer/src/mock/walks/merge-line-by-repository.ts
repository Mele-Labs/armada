// On All repositories the Cockpit's merge line is dimmed and says to select a repository, until a Job, a Session or a pull
// request is open on the stage: then it is that one's repository, and one picked draws its own. Over `merge-line-by-repository`.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Work", { exact: true });
const queue = region("Merge queue");

const mergeLineByRepository = walk("merge-line-by-repository", [
  { look: inside(queue, text("Select a repo to see PRs")), say: "On All with nothing open the merge line is dimmed, with no main light and no chips. The merge icon stays" },
  { press: inside(list, button("Trim the feed")), say: "Open a Job in armada" },
  { look: inside(queue, button(/^#2101/)), say: "The line is armada's: its main and its pull requests" },
  { press: inside(list, button("Lay out the invoice PDF")), say: "Open a Session in storefront" },
  { look: inside(queue, button(/^#3011/)), say: "The line is storefront's now, and armada's pull requests are gone from it" },
  { press: button("Repository"), say: "Pick one repository" },
  { press: role("menuitem", "armada"), say: "armada" },
  { look: inside(queue, button(/^#2102/)), say: "One picked draws that repository's line, whatever is open" },
]);

export { mergeLineByRepository as "merge-line-by-repository" };
