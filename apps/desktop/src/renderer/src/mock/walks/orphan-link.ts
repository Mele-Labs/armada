// A Job whose starter has ended: no link and no connector while the starter is out of the list, and both once All lists it.
// Over the `orphan-link` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });

export const orphanLink = walk("orphan-link", [
  { look: deckRow("Trim the retry loop"), say: "The Job that Plan the retry work started. That one has ended and is not in the list, so this is a plain row with no link and no connector" },
  { press: inside(list, role("button", /^Filter/)), say: "Filter" },
  { press: role("menuitem", "All"), say: "All lists the ended Jobs too" },
  { look: inside(list, role("img", "Linked Job")), say: "The starter is listed, and the link and the connector are back on the Job under it" },
]);
