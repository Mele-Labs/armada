// The 29 Oct 2026 notes on Workspaces, lifecycles and rules, each change straight to: a pick opens the side panel, a repository that differs is marked, the header is icons, the lanes
// are edited by hand on a copy of your own, a lane is any lifecycle, the Job's Lanes opens in the side panel with the Now beat saying what the agent is doing, a rule waiting on you
// is answered there, and a comment's triage is on the comment in the pull request. The long walk is `workspaces-lanes`. A mock, no Fleet behind it.

import { button, deckRow, inside, region, role, text, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Lanes", { exact: true });
const PANEL = role("dialog");

const views = role("toolbar", "Views", { exact: true });
const LANES_PANEL = role("dialog", "Lanes", { exact: true });
const JOB_LANES = inside(LANES_PANEL, group("Lanes"));
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));
const RULE_PANEL = role("dialog", "Rule: Ask for review");
const PR_PANEL = role("dialog", /#812/);

const notes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "A node opens the side panel. Armada's copy is read only, so its toolbar says Make my copy" },
  { look: inside(PANEL, text("Peer Review")), say: "Where a repository differs the node carries a mark, and the panel lists each repository's value beside the Workspace's" },
  { press: inside(role("toolbar", "Edit Ticket In review"), button("Make my copy")), say: "The first edit makes your copy" },
  { look: inside(role("toolbar", "Acts"), group("Where it is kept")), say: "The header is icons with tooltips, in three groups: compare and reset, keep, and delete apart" },
  { drag: role("button", "Connect from step Build"), by: { x: 112, y: 263 }, say: "A connector dragged from one node to another draws a rule. A step to a state is a rule that asks" },
  { look: inside(PANEL, role("radio", "Asks")), say: "Its fields open in the panel" },
  { press: inside(LANES, button("Step: Open PR", { exact: true })), say: "A pick opens the panel and a toolbar over the node" },
  { press: button("Add after step Open PR"), say: "Edit, Tell an agent, Add after and Delete. Add after puts a step beside it" },
  { press: inside(group("Armada"), button("planning", { exact: true })), say: "A lane is any lifecycle" },
  { look: inside(LANES, button("Document: In review", { exact: true })), say: "planning moves a Document through Draft, In review and Approved, with a ticket beside it" },

  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job" },
  { look: text("Reading ARM-41 and ARM-42"), say: "The Now beat says what the agent is doing" },
  { press: inside(views, button("Lanes", { exact: true })), say: "Lanes opens in the side panel, fitted to its width" },
  { later: JOB_LANES, say: "Panned to where the Job is" },
  { later: node("ARM-41: In progress, assigned to you"), say: "A ticket's state is a pin on a heavier frame. Only the step the agent works on takes the accent" },
  { later: node("PR #812: Draft"), say: "Two moments on" },
  { press: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "A rule waiting on you opens what it needs in the panel" },
  { press: inside(RULE_PANEL, button("Send", { exact: true })), say: "The card is the rule's own: where it goes and to whom. Send sends it" },
  { press: inside(RULE_PANEL, button("Back")), say: "Back to the lanes" },
  { later: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The ask went out" },
  { later: inside(LANES_PANEL, region("Comments on PR #812")), say: "Comments are a monitor: what the agent made of each" },
  { press: inside(LANES_PANEL, button(/^Open in the pull request: .* on src\/cart\/totals\.ts/)), say: "A comment opens the pull request at that comment" },
  { look: inside(PR_PANEL, button("Send to Job: Maya on src/cart/totals.ts:42", { exact: true })), say: "The triage is a mark on the comment, with its acts" },
]);

export { notes as "workspaces-lanes-notes" };
