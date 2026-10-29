// Workspaces, lifecycles and rules (28 Oct 2026): System › Workflows draws the Workspace my-store as lanes, the feature workflow over the Ticket, PR and Deploy
// lanes with rules as the edges between them, and a Job's Lanes view draws the same lanes with one for each ticket, pull request and deploy it holds, moment
// by moment, until the deploy is healthy and the tickets are closed. Ending a Job that still holds a deploy lists what is in flight first.
// A mock, no Fleet behind it. The design of 28 Oct, Workspaces, lifecycles and rules.

import { button, deckRow, dialog, inside, region, role, tab, text, toSystemPage, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const YOURS = group("Yours");
const SCOPE = group("Scope");
const LANES = region("Lanes", { exact: true });
const INSPECTOR = role("complementary", "Inspector");
const PROPOSED = region("Proposed change");
const ASK = role("textbox", "Tell an agent what to change");

const views = role("toolbar", "Views", { exact: true });
const lanes = button("Lanes", { exact: true });
const JOB_LANES = region("Lanes", { exact: true });
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));

export const workspacesLanes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: LANES, say: "my-store: the feature workflow on top, and a lane each for Ticket, PR and Deploy" },
  { look: inside(SCOPE, tab("store-api", { exact: true })), say: "Two repositories, and the scope that picks the Workspace or one of them" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "The Ticket lifecycle's In review state" },
  { look: inside(INSPECTOR, text("Code Review, In Review")), say: "Jira's Code Review and In Review count as In review" },
  { press: inside(SCOPE, tab("store-api", { exact: true })), say: "store-api, which has words of its own" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review again, read for store-api" },
  { look: inside(INSPECTOR, text("Peer Review")), say: "Here Jira's Peer Review is In review" },
  { press: inside(SCOPE, tab("my-store", { exact: true })), say: "Back to the Workspace" },
  { press: inside(LANES, button("Move to In review", { exact: true })), say: "A rule, the edge from the PR lane to the Ticket lane" },
  { look: inside(INSPECTOR, role("radio", "Acts")), say: "When the PR opens, move the ticket to In review. Autonomy is Acts" },
  { key: "r", on: role("group", "Lanes of feature"), say: "R goes round the rules" },
  { press: inside(YOURS, button("feature", { exact: true })), say: "Yours, a copy that can be changed" },
  { type: "close the ticket once merged and the deploy is healthy", into: ASK, say: "Tell an agent what to change" },
  { press: button("Send to agent"), say: "Send" },
  { look: inside(PROPOSED, group("Close the ticket")), say: "The rule it drew up: when Deploy is Healthy, move the ticket to Done" },
  { look: inside(LANES, button("Proposed: Close the ticket", { exact: true })), say: "Drawn on the canvas as a diff, a dashed edge from Deploy to Ticket" },
  { press: inside(PROPOSED, button("Approve", { exact: true })), say: "Approve adds it to the Workspace" },
  { look: inside(LANES, button("Close the ticket", { exact: true })), say: "The rule is one of the Workspace's" },

  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job dispatched with ARM-41 and ARM-42" },
  { press: inside(views, lanes), say: "Lanes, beside the Job's other views" },
  { later: JOB_LANES, say: "The workflow on top and a lane for each ticket. Both are To do, and the rule that moves them waits for Plan to start" },
  { later: node("ARM-41: In progress"), say: "Plan starts. The tickets move to In progress and are assigned to you" },
  { later: node("PR #812: Draft"), say: "The PR opens and gets its own lane. The tickets move to In review" },
  { later: text("Review ask for PR #812"), say: "Reviewers are assigned, and the review ask waits as a draft for your press" },
  { later: inside(JOB_LANES, region("Comments on PR #812")), say: "Comments arrive. Triage marks two as needing a change and one as not worth acting on" },
  { later: text("Waiting on PR #812: Approved"), say: "Land waits on the PR being approved" },
  { later: node("Deploy: Rolling out"), say: "Merged. The deploy lane starts, rolling out" },
  { look: inside(JOB_LANES, button("Close the ticket", { exact: true })), say: "The deploy is healthy, the tickets are closed, and the Job is done" },
  { press: inside(views, lanes), say: "Back to the thread" },
  { type: "skip the deploy watch on this one\n", into: role("textbox", "Message"), say: "Skip the deploy watch, typed to the agent on this Job" },
  { press: inside(views, lanes), say: "Lanes again" },
  { look: inside(JOB_LANES, role("img", /^Changed on this Job/)), say: "The Deploy lane marks the change" },

  { press: inside(views, lanes), say: "Back to the thread" },
  { press: deckRow("Fix cart rounding"), say: "A second Job, with its deploy still rolling out" },
  { press: inside(role("region", /^Job: /), button("More")), say: "End job is behind the three dots" },
  { press: role("menuitem", "End job"), say: "End lists what is in flight first" },
  { look: dialog("In flight"), say: "Deploy is rolling out and ARM-52 is in review. Ending the Job ends them" },
]);

export { workspacesLanes as "workspaces-lanes" };
