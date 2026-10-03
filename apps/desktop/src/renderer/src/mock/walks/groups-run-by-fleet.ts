// A plan Fleet ran in groups (spike 022, slice 2): its groups are Fleet's, the
// red group's task carries the failed mark and says why, a Check on a passed
// group opens that group's own run, and Restart this task works the task again.

import { button, dialog, inside, role, tab, walk } from "../walk";

const GROUPS = role("list", "Groups, in the order they run");
const groupOne = inside(GROUPS, role("listitem", "Group 1", { exact: true }));
const t4 = inside(GROUPS, role("listitem", /^T4 /));

const OVERVIEW_PLAN = role("region", "Plan", { exact: true });

export const groupsRunByFleet = walk("real/groups-run-by-fleet", [
  { hover: inside(OVERVIEW_PLAN, role("img", "Failed", { exact: true })), say: "Overview's rows draw a group's state as the same mark" },
  { press: tab("Plan"), say: "Fleet's plan, in its own three groups" },
  { press: tab("List"), say: "Groups one and two passed, each with its commit" },
  { hover: inside(groupOne, role("img", "Passed", { exact: true })), say: "A group's state is a mark, named on hover" },
  { hover: inside(t4, role("img", "Failed", { exact: true })), say: "T4 failed: group three was still red on its last run" },
  { press: inside(groupOne, button("test, passed", { exact: true })), say: "Group one's test, a segment of its Checks, opens its log" },
  { press: inside(dialog("Check log"), button("Open in the Record")), say: "Its own run, though group three ran test too" },
  { look: role("heading", "test", { exact: true }), say: "Group one's run: it passed, and held nothing back" },
  { press: tab("Plan"), say: "Back to the plan" },
  { press: inside(t4, button(/Answer restart and move/)), say: "The failed task's panel" },
  { press: button("Restart this task"), say: "Restart this task works it again" },
]);
