// A rule or lifecycle with a Keep for every Job draft waiting on Approve wears the clock, and one the repository does not hold yet is drawn from its draft (armada-7gs). Over the `workspaces-pieces-keeps` scenario.

import { button, inside, region, role, tab, toSystemPage, walk } from "../walk";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Workflow", { exact: true });
const KEPT = "Kept for every Job once its draft is approved";

const workflowsPendingKeep = walk("workspaces-pieces-keeps", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: inside(group("Runs in"), tab("store-web", { exact: true })), say: "store-web's tab" },
  { hover: inside(LANES, button("Start the ticket", { exact: true })), say: "A held rule with a draft: the clock" },
  { hover: inside(inside(LANES, group("Ticket")), role("img", KEPT, { exact: true })), say: "A held lifecycle with a draft: the clock" },
  { look: inside(LANES, button("Proposed: Move to review", { exact: true })), say: "A rule not held yet, drawn from its draft" },
  { look: inside(LANES, group("QA sign-off")), say: "A lifecycle not held yet, drawn from its draft" },
], { width: 2000, height: 1200 });

export { workflowsPendingKeep as "workflows-pending-keep" };
