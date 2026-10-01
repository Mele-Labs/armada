// A group that failed at its checks says which check, what it expected, and
// which task stopped and why, in the group's own panel.

import { card, region, tab, text, walk } from "../walk";

export const whyAGroupFailed = walk("arc/group-failed", [
  { press: tab("Plan"), say: "Group 3 is running again" },
  { press: card("Group 3"), say: "Its panel says why" },
  { look: region("Checks at this boundary"), say: "The checks it ran, failed first" },
  { look: text("1 of 1384 failed"), say: "What the failed check found" },
  { look: text("The row's press opened the Board"), say: "The task that stopped, and why" },
]);
