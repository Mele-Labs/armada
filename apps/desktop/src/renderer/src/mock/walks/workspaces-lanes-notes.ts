// The 29 Oct 2026 notes on Workspaces, lifecycles and rules, each change straight to: a pick opens the side panel, a repository that differs is marked, the header is icons, the lanes
// are edited by hand on a copy of your own, a lane is any lifecycle, the Job's Lanes opens in the side panel with the Now beat saying what the agent is doing, a rule waiting on you
// is answered there, and a comment's triage is on the comment in the pull request. A third Job changes its own copy of the workflow and starts two Jobs; Lanes opens in the side panel
// on a window narrower than 1800px and in the spine on a wider one. The long walk is `workspaces-lanes`. A mock, no Fleet behind it.

import { button, deckRow, inside, region, role, text, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const LANES = region("Lanes", { exact: true });
const PANEL = role("dialog");
const RUNS_IN = group("Runs in");
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));

const views = role("toolbar", "Views", { exact: true });
const LANES_PANEL = role("dialog", "Lanes", { exact: true });
const JOB_LANES = inside(LANES_PANEL, group("Lanes"));
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));
const RULE_PANEL = role("dialog", "Rule: Ask for review");
const PR_PANEL = role("dialog", /#812/);
const SPINE = region("Lanes", { exact: true });
const SPINE_LANES = inside(SPINE, group("Lanes"));
const inSpine = (name: string) => inside(SPINE_LANES, button(name, { exact: true }));

const notes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: RUNS_IN, say: "Runs in: all of my-store or one repository. A repository's tab draws the lanes as the workflow runs there" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "A node opens the side panel. Armada's copy is read only, so its toolbar says Make my copy" },
  { look: inside(PANEL, text("Code Review, In Review")), say: "The panel reads one scope at a time: Jira statuses for all of my-store" },
  { press: tab("store-api"), say: "store-api's tab. The node that differs there carries a mark" },
  { look: inside(PANEL, text("Changed for store-api")), say: "Peer Review is store-api's own, marked as changed for store-api" },
  { press: tab("All of my-store"), say: "Back to the base" },
  { press: inside(role("toolbar", "Edit Ticket In review"), button("Make my copy")), say: "The first edit makes your copy" },
  { look: inside(role("toolbar", "Acts"), group("Where it is kept")), say: "The header is icons with tooltips, in three groups: compare and reset, keep, and delete apart" },
  { press: tab("store-api"), say: "On your copy, store-api's tab" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review, marked" },
  { press: inside(PANEL, button("Use my-store's", { exact: true })), say: "Use my-store's drops store-api's own value. Edits on a repository's tab change that repository only" },
  { look: inside(PANEL, text("Code Review, In Review")), say: "store-api reads my-store's again" },
  { press: tab("All of my-store"), say: "Back to the base" },
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

  { press: inside(PR_PANEL, button("Back")), say: "Back to the lanes" },
  { press: inside(LANES_PANEL, button("Close")), say: "Close the panel" },
  { press: deckRow("Go through app feedback"), say: "A Job that goes through app feedback" },
  { press: inside(views, button("Lanes", { exact: true })), say: "Lanes opens in the side panel on a window narrower than 1800px" },
  { later: JOB_LANES, say: "Lanes opens on the workflow as the Job started it" },
  { look: node("Step: Oversee"), say: "The agent changed its own copy: Walk and Refine skipped, Dispatch and Oversee added after Approve plan. Each is marked" },
  { later: node("Step: Oversee"), say: "Dispatch starts a Job for each fix, and Oversee waits for them" },
  { look: node("Fix cart rounding: PR open"), say: "Two Jobs, each a lane of its own with Started, PR open and Landed" },
  { look: inside(JOB_LANES, text("Waiting on: Fix cart rounding, Tidy the PR panel landing")), say: "Oversee names the Jobs it waits on, and ends when the last lands" },
  { press: inside(LANES_PANEL, button("Close")), say: "Close the panel" },
  { later: inside(views, button("Lanes", { exact: true })), say: "The next steps play a window wider than 1800px" },
  { press: inside(views, button("Lanes", { exact: true })), say: "Wider than 1800px, Lanes opens in the spine column" },
  { look: inside(SPINE_LANES, text("Waiting on Tidy the PR panel landing: Landed")), say: "It is a view like Artifacts and Checks: pressed again, or Esc, shuts it" },
  { later: inSpine("Tidy the PR panel landing: PR open"), say: "One Job has landed and one is in review" },
  { look: inSpine("Tidy the PR panel landing: Landed"), say: "Both landed. Oversee is done, and the two Jobs' Landed nodes are in view" },
]);

export { notes as "workspaces-lanes-notes" };
