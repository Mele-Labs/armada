// Workspaces, lifecycles and rules (29 Oct 2026): System › Workflows draws the Workspace my-store as lanes, a workflow over the lifecycles it moves, with rules as the edges
// between them. What is picked opens in the side panel, and the lanes are edited by hand or by telling an agent. A Job's Lanes opens in the same panel and draws the same lanes
// as its tickets, pull request and deploy move, until the deploy is healthy and the tickets are closed. Ending a Job that still holds a deploy lists what is in flight first.
// A mock, no Fleet behind it. The design of 28 Oct, Workspaces, lifecycles and rules.

import { button, deckRow, dialog, inside, region, role, text, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const YOURS = group("Yours");
const LANES = region("Lanes", { exact: true });
const PANEL = role("dialog");
const PROPOSED = region("Proposed change");
const ASK = role("textbox", "Tell an agent what to change");

const views = role("toolbar", "Views", { exact: true });
const lanes = button("Lanes", { exact: true });
const LANES_PANEL = dialog("Lanes");
const JOB_LANES = inside(LANES_PANEL, group("Lanes"));
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));
const RULE_PANEL = dialog("Rule: Ask for review");
const PR_PANEL = role("dialog", /#812/);
const SEND_MAYA = button("Send to Job: Maya on src/cart/totals.ts:42", { exact: true });

export const workspacesLanes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: LANES, say: "my-store: the feature workflow on top, and a lane each for Ticket, PR and Deploy" },
  { look: inside(LANES, role("img", "Differs in store-api")), say: "A mark on In review: one repository reads it differently" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "The Ticket lifecycle's In review state, in the side panel" },
  { look: inside(PANEL, text("Code Review, In Review")), say: "Jira's Code Review and In Review count as In review for my-store" },
  { look: inside(PANEL, text("Peer Review")), say: "store-api's own value, beside it: Peer Review" },
  { press: inside(LANES, button("Move to In review", { exact: true })), say: "A rule, the edge from the PR lane to the Ticket lane" },
  { look: inside(PANEL, role("radio", "Acts")), say: "When the PR opens, move the ticket to In review. Autonomy is Acts" },
  { key: "]", on: role("group", "Lanes of feature"), say: "] goes to the next rule, and [ to the one before" },
  { look: inside(PANEL, role("radio", "Asks")), say: "The panel follows to Ask for review: when the PR has reviewers on, draft the ask. Autonomy is Asks" },
  { press: inside(YOURS, button("feature", { exact: true })), say: "Yours, a copy that can be changed" },
  { type: "close the ticket once merged and the deploy is healthy", into: ASK, say: "Tell an agent what to change" },
  { press: button("Send to agent"), say: "Send" },
  { look: inside(PROPOSED, group("Close the ticket")), say: "The rule it drew up: when Deploy is Healthy, move the ticket to Done" },
  { look: inside(LANES, button("Proposed: Close the ticket", { exact: true })), say: "Drawn on the canvas as a diff, a dashed edge from Deploy to Ticket" },
  { press: inside(PROPOSED, button("Approve", { exact: true })), say: "Approve adds it to the Workspace" },
  { look: inside(LANES, button("Close the ticket", { exact: true })), say: "The rule is one of the Workspace's" },

  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job dispatched with ARM-41 and ARM-42" },
  { press: inside(views, lanes), say: "Lanes, beside the Job's other views, opens in the side panel" },
  { later: JOB_LANES, say: "The workflow on top and a lane for each ticket. Both are To do, and the rule that moves them waits for Plan to start" },
  { later: node("ARM-41: In progress, assigned to you"), say: "Plan starts. The tickets move to In progress and are assigned to you" },
  { later: node("PR #812: Draft"), say: "The PR opens and gets its own lane. The tickets move to In review" },
  { look: node("PR #812: Reviewers on"), say: "Reviewers are on the PR, and the review ask waits for your press" },
  { hover: node("PR #812: Reviewers on"), say: "The reviewers are named in its tooltip" },
  { press: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The rule waiting on you opens what it needs in the panel" },
  { look: inside(RULE_PANEL, text("Maya and Jun")), say: "The ask is the rule's: where it goes and to whom" },
  { press: inside(RULE_PANEL, button("Send", { exact: true })), say: "Send sends the review ask" },
  { press: inside(RULE_PANEL, button("Back")), say: "Back to the lanes" },
  { later: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The ask went out, so its rule reads as acted" },
  { later: inside(LANES_PANEL, region("Comments on PR #812")), say: "Comments arrive. Triage marks two as needing a change and one as not worth acting on" },
  { press: inside(LANES_PANEL, button(/^Open in the pull request: .* on src\/cart\/totals\.ts/)), say: "A comment opens the pull request at that comment" },
  { look: inside(PR_PANEL, SEND_MAYA), say: "The triage is on the comment itself: a mark for what it needs, where in the code, and its act" },
  { press: inside(PR_PANEL, SEND_MAYA), say: "Send to Job hands the comment to the agent" },
  { look: inside(PR_PANEL, role("img", "Sent to the Job")), say: "A check says it went" },
  { press: inside(PR_PANEL, button("Back")), say: "Back to the lanes" },
  { later: text("Waiting on PR #812: Approved"), say: "Land waits on the PR being approved" },
  { later: node("Deploy: Rolling out"), say: "Merged. The deploy lane starts, rolling out" },
  { look: inside(JOB_LANES, button("Close the ticket", { exact: true })), say: "The deploy is healthy and the tickets are closed. Every step is done, and the Job's row reads done" },
  { type: "skip the deploy watch on this one\n", into: role("textbox", "Message"), say: "Skip the deploy watch, typed to the agent on this Job" },
  { look: inside(JOB_LANES, role("img", /^Changed on this Job/)), say: "The Rolling out state is marked as changed on this Job, with a dashed frame" },

  { press: inside(LANES_PANEL, button("Close")), say: "Close the panel" },
  { press: deckRow("Fix cart rounding"), say: "A second Job, with its deploy still rolling out" },
  { press: inside(role("region", /^Job: /), button("More")), say: "End job is behind the three dots" },
  { press: role("menuitem", "End job"), say: "End lists what is in flight first" },
  { look: dialog("In flight"), say: "Deploy is rolling out and ARM-52 is in review. Ending the Job ends them" },
]);

export { workspacesLanes as "workspaces-lanes" };
