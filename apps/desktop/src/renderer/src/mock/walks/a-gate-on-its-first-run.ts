// A group's gate on its first run (the owner's Job 13, 7 Oct 2026). Fleet recorded two of
// the Checks as skipped; Bridge drew them failed while the gate ran. A skipped Check reads
// skipped with its reason on hover, a running one reads running, and the group is not red.

import { button, inside, region, role, tab, text, walk } from "../walk";

const GROUPS = role("list", "Groups, in the order they run");
const groupOne = inside(GROUPS, role("listitem", "Group 1", { exact: true }));
const checks = inside(groupOne, region("Checks at this boundary"));

export const aGateOnItsFirstRun = walk("checks/gate-first-run", [
  { press: tab("Plan"), say: "The plan, one group, its gate running" },
  { press: tab("List"), say: "Group 1 and its Checks" },
  { press: inside(checks, button(/^Checks/)), say: "Every Check of this run" },
  { hover: inside(checks, text("build")), say: "Skipped says why on hover, and is neither passed nor failed" },
  { look: inside(checks, text("desktop_test")), say: "Running until its row lands" },
]);
