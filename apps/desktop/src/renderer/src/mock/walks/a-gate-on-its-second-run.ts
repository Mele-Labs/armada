// A group's gate on its second run, the first run red (the owner's Job 13, 7 Oct 2026). The
// collapsed line and the grid read the live run alone: the first run's desktop_test red is
// not carried over, and the Check still running reads running. Then the same Job's Record
// and the Checks page, which held nothing.

import { button, inside, region, role, tab, text, walk } from "../walk";

const GROUPS = role("list", "Groups, in the order they run");
const groupOne = inside(GROUPS, role("listitem", "Group 1", { exact: true }));
const checks = inside(groupOne, region("Checks at this boundary"));

export const aGateOnItsSecondRun = walk("checks/gate-second-run", [
  { press: tab("Plan"), say: "The plan, its gate on a second run" },
  { press: tab("List"), say: "Group 1: the live run, not the first run's red" },
  { look: inside(checks, role("button", /^Checks/)), say: "The line says what the grid says" },
  { press: inside(checks, button(/^Checks/)), say: "Every Check of the live run" },
  { look: inside(checks, text("desktop_test")), say: "Running, not the first run's failed" },
  { hover: inside(checks, text("build")), say: "Skipped, not passed, and says why" },
  { press: tab("Record"), say: "The Job's Record holds its Checks" },
  { look: role("table"), say: "One row per Check run" },
]);
