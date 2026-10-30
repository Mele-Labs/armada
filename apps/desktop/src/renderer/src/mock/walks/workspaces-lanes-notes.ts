// The 29 Oct 2026 notes on Workspaces, lifecycles and rules, each change straight to: a pick opens the side panel with its settings as fields saved as they are changed, Runs in opens on a
// Default tab and a tab that differs from the level above carries the changed mark, the box that tells an agent what to change stands under the tabs, the first change on Armada's workflow makes your own, the canvas draws each kind in its own shape,
// the Job's Workflow opens in the side panel with the Now beat opening the rule that waits on you, and a comment's triage is on the comment in the pull request. A third Job changes its
// own copy of the workflow and starts two Jobs; Workflow opens: in the side panel under 1800px, in the spine from it and in the spine on a wider one. The long walk is `workspaces-lanes`.
// A mock, no Fleet behind it.

import { button, deckRow, either, inside, region, role, text, toSystemPage, walk } from "../walk";
import type { Target } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Workflow", { exact: true });
const PANEL = role("dialog");
const YOURS = group("Yours");
const RUNS_IN = group("Runs in");
const ASK = role("textbox", "Tell an agent what to change");
const statuses = () => role("textbox", "Jira statuses", { exact: true });
const dropdown = (name: string) => role("combobox", name, { exact: true });
const closePanel = { press: inside(PANEL, button("Close")), say: "Close the panel" } as const;
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));

const views = role("toolbar", "Views", { exact: true });
const workflowView = button("Workflow", { exact: true });
// Workflow is a dialog on a window narrower than 1800px and a section of the spine on a wider one, so what is inside it is found without naming which.
const JOB_LANES = group("Workflow");
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));
const RULE_PANEL = role("dialog", "Rule: Ask for review");
const PR_PANEL = role("dialog", /#812/);
// What goes back to the Workflow: a button where the panel opened over the Workflow's own, and Close where it opened over the docked one.
const back = (panel: Target) => either(inside(panel, button(/^Back/)), inside(panel, button("Close")));
const SPINE = region("Workflow", { exact: true });
const SPINE_LANES = inside(SPINE, group("Workflow"));
const NOW = role("list", "Now", { exact: true });
const inSpine = (name: string) => inside(SPINE_LANES, button(name, { exact: true }));

const notes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: RUNS_IN, say: "Runs in opens on Default, the workflow as it runs anywhere no Workspace or repository changed it. Edits here change every Workspace and repository that keeps the default" },
  { look: ASK, say: "Tell an agent what to change stands under the title and the tabs, in view without scrolling" },
  { look: inside(LANES, role("img", "Start", { exact: true })), say: "The canvas draws each kind its own way: the workflow starts and ends at small circles, and a step is a rounded box" },
  { look: inside(LANES, button("Deploy: Rolling out", { exact: true })), say: "A tracker or forge state is a pill, a watch is a hexagon and a rule is a diamond. Each kind has a tint of its own, and a dashed edge is a wait" },
  { press: inside(LANES, button("Ticket: To do", { exact: true })), say: "A node opens the side panel over the canvas" },
  { press: inside(role("toolbar", "Edit Ticket To do"), button("Edit Ticket To do", { exact: true })), say: "Edit, on the node, goes to its fields" },
  { type: "To Do, Backlog, Icebox\n", into: inside(PANEL, statuses()), say: "The first change on Default makes yours, and nothing says so" },
  { look: inside(YOURS, button("feature", { exact: true })), say: "The change is kept under Yours" },
  { look: inside(role("toolbar", "Acts"), group("Where it is kept")), say: "The header is icons with tooltips, in three groups: compare and reset, keep, and delete apart" },
  closePanel,
  { press: tab("All of my-store"), say: "All of my-store carries the changed mark: it differs from Default. Edits here change my-store's repositories that keep my-store's" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review for all of my-store" },
  { look: inside(PANEL, statuses()), say: "Jira statuses is a field in the panel, and it saves as it changes" },
  { look: inside(PANEL, text("Changed for my-store")), say: "Marked as changed for my-store, with a way back to the default" },
  closePanel,
  { press: tab("store-api"), say: "store-api's tab. Edits here change store-api only, and the node that differs there carries a mark" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review, marked" },
  { look: inside(PANEL, text("Changed for store-api")), say: "Peer Review is store-api's own, marked as changed for store-api, with a way back to my-store's" },
  { press: inside(PANEL, button("Use my-store's", { exact: true })), say: "Use my-store's drops store-api's own value. Edits on a repository's tab change that repository only" },
  { look: inside(PANEL, statuses()), say: "store-api reads my-store's again" },
  closePanel,
  { press: inside(LANES, button("Step: Open PR", { exact: true })), say: "Adding, deleting and connecting work on every tab. Open PR, on store-api" },
  { press: button("Add after step Open PR"), say: "A step added on store-api runs in store-api only" },
  { look: tab("store-api"), say: "store-api carries the changed mark again: it has a step my-store does not" },
  closePanel,
  { press: tab("Default"), say: "Default does not draw that step. What is added on Default belongs to the workflow, for every Workspace and repository" },
  { drag: role("button", "Connect from step Build"), by: { x: 112, y: 263 }, say: "A connector dragged from one node to another draws a rule. A step to a state is a rule that asks" },
  { look: inside(PANEL, role("radio", "Asks")), say: "Its fields open in the panel" },
  { look: inside(PANEL, dropdown("When")), say: "When is two dropdowns, the step or state and then the moment: Build starts" },
  { type: "ends", into: inside(PANEL, dropdown("When moment")), say: "Choosing ends saves at once: Build ends" },
  { press: inside(LANES, button("Step: Open PR", { exact: true })), say: "Another node switches the panel to it, and a toolbar stands over the node" },
  { look: inside(PANEL, dropdown("Ends when")), say: "Ends when is two dropdowns, set from the step: The pull request, is open" },
  { type: "approved", into: inside(PANEL, dropdown("Ends when state")), say: "Choosing is approved saves at once: Open PR ends when the pull request is approved" },
  { press: button("Add after step Open PR"), say: "Edit, Tell an agent, Add after and Delete. Add after puts a step beside it" },
  { press: inside(group("Armada"), button("planning", { exact: true })), say: "A lane is any lifecycle" },
  { look: inside(LANES, button("Document: In review", { exact: true })), say: "planning moves a Document through Draft, In review and Approved, with a ticket beside it" },

  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job" },
  { look: text("Reading ARM-41 and ARM-42"), say: "The Now beat says what the agent is doing" },
  { press: inside(views, workflowView), say: "Workflow opens, fitted to its width" },
  { later: JOB_LANES, say: "Panned to where the Job is" },
  { later: node("ARM-41: In progress, assigned to you"), say: "A ticket's state is a pin on a heavier frame. Only the step the agent works on takes the accent" },
  { later: node("PR #812: Draft"), say: "Two moments on" },
  { key: "Escape", on: JOB_LANES, say: "Esc shuts Workflow" },
  { press: inside(NOW, button("Ask for review", { exact: true })), say: "Waiting on you, in Now, opens the same rule in the side panel as the canvas does" },
  { press: inside(RULE_PANEL, button("Send", { exact: true })), say: "The card is the rule's own: where it goes and to whom. Send sends it" },
  { look: inside(RULE_PANEL, role("img", "Sent", { exact: true })), say: "The panel stays open and says it went" },
  { press: inside(RULE_PANEL, button("Close")), say: "Close the panel" },
  { press: inside(views, workflowView), say: "Workflow opens again" },
  { later: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The ask went out" },
  { later: region("Comments on PR #812"), say: "Comments are a monitor: what the agent made of each" },
  { press: button(/^Open in the pull request: .* on src\/cart\/totals\.ts/), say: "A comment opens the pull request at that comment" },
  { look: inside(PR_PANEL, button("Send to Job: Maya on src/cart/totals.ts:42", { exact: true })), say: "The triage is a mark on the comment, with its acts" },

  { press: back(PR_PANEL), say: "Back to the Workflow" },
  { key: "Escape", on: JOB_LANES, say: "Esc shuts Workflow" },
  { press: deckRow("Go through app feedback"), say: "A Job that goes through app feedback" },
  { press: inside(views, workflowView), say: "Workflow opens: in the side panel under 1800px, in the spine from it" },
  { later: JOB_LANES, say: "The workflow as the Job started it" },
  { look: node("Step: Oversee"), say: "The agent changed its own copy: Walk and Refine skipped, Dispatch and Oversee added after Approve plan. Each is marked" },
  { later: node("Step: Oversee"), say: "Dispatch starts a Job for each fix, and Oversee waits for them" },
  { look: node("Fix cart rounding: PR open, ticket ARM-52, In review"), say: "Two Jobs, each a card with the Drone on a lane of its own: Started, PR open and Landed" },
  { look: inside(JOB_LANES, text('Waiting on Job "Fix cart rounding", Job "Tidy the PR panel landing": Landed')), say: "Oversee names the Jobs it waits on, and ends when the last lands" },
  { key: "Escape", on: JOB_LANES, say: "Esc shuts Workflow" },
  { later: inside(views, workflowView), say: "From here the window plays wider than 1800px" },
  { press: inside(views, workflowView), say: "Wider than 1800px, Workflow opens in the spine column" },
  { look: inside(SPINE_LANES, text("Waiting on Tidy the PR panel landing: Landed")), say: "It is a view like Artifacts and Checks: pressed again, or Esc, shuts it" },
  { later: inSpine("Tidy the PR panel landing: PR open"), say: "One Job has landed and one is in review" },
  { look: inSpine("Tidy the PR panel landing: Landed"), say: "Both landed. Oversee is done, and the two Jobs' Landed nodes are in view" },
]);

export { notes as "workspaces-lanes-notes" };
