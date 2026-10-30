// A Job that started other Jobs for its tickets hands each ticket to the Job working it. On the parent's Workflow, the started Job's lane shows the ticket it carries and where it stands,
// and the parent has no lane for that ticket. A ticket the parent kept, with no Job started for it, keeps its own. The head of that Job's lane opens its own Workflow on a double-click or Enter, and Back returns.
// Over the scenario of the same name, `workspaces-lanes` from the moment both Jobs have started. A walk has no double-click, so its press is the click with no pointer behind it that Enter makes.

import { button, deckRow, inside, role, walk } from "../walk";

const views = role("toolbar", "Views", { exact: true });
const lanes = role("group", "Workflow", { exact: true });
const cart = inside(lanes, button("Fix cart rounding: PR open, ticket ARM-52, In review", { exact: true }));
const kept = inside(lanes, button("ARM-58: To do", { exact: true }));
const head = inside(lanes, button("Open Workflow: Fix cart rounding", { exact: true }));
const ticket = inside(lanes, button("ARM-52: In review, assigned to you", { exact: true }));

const ticketOnJobLane = walk("ticket-on-job-lane", [
  { press: deckRow("Go through app feedback"), say: "A Job that started two Jobs, one for a ticket" },
  { press: inside(views, button("Workflow", { exact: true })), say: "Its Workflow" },
  { look: cart, say: "The Fix cart rounding card shows its ticket, ARM-52. This Job has no lane for that ticket" },
  { hover: cart, say: "The tooltip names the ticket and its state, and says the Job holds it" },
  { look: kept, say: "ARM-58 has no Job started for it, so it keeps its lane" },
  { press: head, say: "A double-click on the head of Fix cart rounding, or Enter on it, opens that Job's own Workflow" },
  { look: ticket, say: "Its own lanes, under its title. ARM-52 and PR #815 are both In review" },
  { press: button("Back to Go through app feedback", { exact: true }), say: "Back returns to the lanes of Go through app feedback as they were" },
]);

export { ticketOnJobLane as "ticket-on-job-lane" };
