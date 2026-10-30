// What a rule did on a Job's lanes (30 Oct 2026): a rule that acted opens in the side panel with each act it planned, for each ticket it was done to, and where the act stands.
// Played at any width: Workflow is a section of the Job's spine on a wide window and a dialog on a narrower one, so no target here names which.
// The acts come over the same read a real Job's do (`get_job_lifecycles`), so the panel is drawn from what Fleet says. A mock, no Fleet behind it.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const views = role("toolbar", "Views", { exact: true });
const workflowView = button("Workflow", { exact: true });
const JOB_LANES = group("Workflow");
const RULE_PANEL = role("dialog", "Rule: Move to In review");

export const workspacesLanesActs = walk("workspaces-lanes", [
  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job dispatched with ARM-41 and ARM-42" },
  { press: inside(views, workflowView), say: "Workflow opens" },
  { later: JOB_LANES, say: "The workflow on top and a lane for each ticket" },
  { later: inside(JOB_LANES, button("ARM-41: In progress, assigned to you", { exact: true })), say: "Plan starts, and the tickets move to In progress" },
  { later: inside(JOB_LANES, button("ARM-41: In review, assigned to you", { exact: true })), say: "The pull request opens, and the tickets move to In review" },
  { press: inside(JOB_LANES, button("Move to In review", { exact: true })), say: "The rule that moved them opens in the side panel" },
  { look: inside(RULE_PANEL, text("Move the ticket to In review")), say: "Each act it planned, and where it stands" },
  { look: inside(RULE_PANEL, region("ARM-42", { exact: true })), say: "One frame for each ticket it was done to" },
]);

export { workspacesLanesActs as "workspaces-lanes-acts" };
