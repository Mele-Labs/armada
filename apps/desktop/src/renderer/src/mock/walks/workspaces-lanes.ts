// Workspaces, lifecycles and rules (29 Oct 2026): System › Workflows draws the Workspace my-store as lanes, a workflow over the lifecycles it moves, with rules as the edges
// between them, on a Default tab first, then all of my-store and each repository. What is picked opens in the side panel, and the lanes are edited by hand or by telling an agent. A Job's Workflow opens in the same panel and draws the same lanes
// as its tickets, pull request and deploy move, until the deploy is healthy and the tickets are closed. Ending a Job that still holds a deploy lists what is in flight first.
// A third Job goes through app feedback and changes its own copy of the workflow, starts two Jobs that each get a lane, and offers `dispatch`; on a window wider than 1800px
// the Workflow opens in the spine instead of the side panel, which the scenario switches on for a step because a walk cannot resize its window.
// A mock, no Fleet behind it. The design of 28 Oct, Workspaces, lifecycles and rules.

import { button, deckRow, dialog, either, inside, region, role, text, toSystemPage, walk } from "../walk";
import type { Target } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const YOURS = group("Yours");
const LANES = region("Workflow", { exact: true });
// The first change on Armada's workflow makes the person's own copy; the settings in the panel are fields, saved as they are changed.
const statuses = () => role("textbox", "Jira statuses", { exact: true });
const PANEL = role("dialog");
const RUNS_IN = group("Runs in");
const tab = (name: string) => inside(RUNS_IN, role("tab", name, { exact: true }));
const PROPOSED = region("Proposed change");
const ASK = role("textbox", "Tell an agent what to change");

const views = role("toolbar", "Views", { exact: true });
const workflowView = button("Workflow", { exact: true });
// Workflow is a dialog on a window narrower than 1800px and a section of the spine on a wider one, so what is inside it is found without naming which.
const JOB_LANES = group("Workflow");
const node = (name: string) => inside(JOB_LANES, button(name, { exact: true }));
const RULE_PANEL = dialog("Rule: Ask for review");
const PR_PANEL = role("dialog", /#812/);
// What goes back to the Workflow: a button where the panel opened over the Workflow's own, and Close where it opened over the docked one.
const back = (panel: Target) => either(inside(panel, button(/^Back/)), inside(panel, button("Close")));
const SEND_MAYA = button("Send to Job: Maya on src/cart/totals.ts:42", { exact: true });
// On a window wider than 1800px the same lanes are a layer in the spine, under the views.
const SPINE = region("Workflow", { exact: true });
const SPINE_LANES = inside(SPINE, group("Workflow"));
const inSpine = (name: string) => inside(SPINE_LANES, button(name, { exact: true }));

export const workspacesLanes = walk("workspaces-lanes", [
  ...toSystemPage("Workflows", "Workflows, from System"),
  { look: LANES, say: "my-store: the feature workflow on top, and a lane each for Ticket, PR and Deploy" },
  { look: RUNS_IN, say: "Runs in opens on Default: the workflow as it runs anywhere no Workspace or repository changed it. Then all of my-store, and a tab for each repository" },
  { look: inside(LANES, role("img", "Differs in my-store, store-api")), say: "A mark on In review: my-store and store-api read it differently" },
  { press: tab("my-store"), say: "my-store carries the changed mark, since it differs from Default. Its line says what an edit there changes" },
  { press: tab("store-api"), say: "store-api's tab draws the lanes as the workflow runs in store-api" },
  { look: inside(LANES, role("img", "Differs in store-api")), say: "In review carries the mark here too: it is store-api's own" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "The Ticket lifecycle's In review state, in the side panel over the canvas" },
  { look: inside(PANEL, statuses()), say: "Jira statuses, a field saved as it is changed: Peer Review in store-api" },
  { look: inside(PANEL, text("Changed for store-api")), say: "Marked as changed for store-api, with a way back to my-store's" },
  { press: inside(PANEL, button("Close")), say: "Close the panel" },
  { press: tab("my-store"), say: "Back to my-store" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review again, for all of my-store" },
  { look: inside(PANEL, statuses()), say: "Code Review and In Review count as In review for my-store" },
  { press: inside(LANES, button("Move to In review", { exact: true })), say: "Another node switches the panel: a rule, the edge from the PR lane to the Ticket lane" },
  { look: inside(PANEL, role("radio", "Acts")), say: "When the PR enters Draft, move the ticket to In review. Autonomy is Acts" },
  { key: "]", on: role("group", "feature workflow"), say: "] goes to the next rule, and [ to the one before" },
  { look: inside(PANEL, role("radio", "Asks")), say: "The panel follows to Ask for review: when the PR has reviewers on, draft the ask. Autonomy is Asks" },
  { press: inside(PANEL, button("Close")), say: "Close the panel" },
  { press: inside(YOURS, button("feature", { exact: true })), say: "Yours, where a change is kept" },
  { press: tab("store-api"), say: "store-api's tab on Yours" },
  { press: inside(LANES, button("Ticket: In review", { exact: true })), say: "In review, marked" },
  { look: inside(PANEL, text("Changed for store-api")), say: "Changed for store-api, with a way back" },
  { press: inside(PANEL, button("Use my-store's", { exact: true })), say: "Use my-store's drops store-api's own value" },
  { look: inside(PANEL, statuses()), say: "store-api reads Code Review and In Review again, and the mark is gone" },
  { press: inside(PANEL, button("Close")), say: "Close the panel" },
  { press: tab("my-store"), say: "Back to my-store" },
  { type: "close the ticket once merged and the deploy is healthy", into: ASK, say: "Tell an agent what to change" },
  { press: button("Send to agent"), say: "Send" },
  { look: inside(PROPOSED, group("Close the ticket")), say: "The rule it drew up: when Deploy is Healthy, move the ticket to Done" },
  { look: inside(LANES, button("Proposed: Close the ticket", { exact: true })), say: "Drawn on the canvas as a diff, a dashed edge from Deploy to Ticket" },
  { press: inside(PROPOSED, button("Approve", { exact: true })), say: "Approve adds it to the Workspace" },
  { look: inside(LANES, button("Close the ticket", { exact: true })), say: "The rule is one of the Workspace's" },

  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job dispatched with ARM-41 and ARM-42" },
  { press: inside(views, workflowView), say: "Workflow, beside the Job's other views, opens" },
  { later: JOB_LANES, say: "The workflow on top and a lane for each ticket. Both are To do, and the rule that moves them waits for Plan to start" },
  { later: node("ARM-41: In progress, assigned to you"), say: "Plan starts. The tickets move to In progress and are assigned to you" },
  { later: node("PR #812: Draft"), say: "The PR opens and gets its own lane. The tickets move to In review" },
  { look: node("PR #812: Reviewers on"), say: "Reviewers are on the PR, and the review ask waits for your press" },
  { hover: node("PR #812: Reviewers on"), say: "The reviewers are named in its tooltip" },
  { press: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The rule waiting on you opens what it needs in the panel" },
  { look: inside(RULE_PANEL, text("Maya and Jun")), say: "The ask is the rule's: where it goes and to whom" },
  { press: inside(RULE_PANEL, button("Send", { exact: true })), say: "Send sends the review ask" },
  { look: inside(RULE_PANEL, role("img", "Sent", { exact: true })), say: "The panel stays open and says it went" },
  { press: back(RULE_PANEL), say: "Back to the Workflow" },
  { later: inside(JOB_LANES, button("Ask for review", { exact: true })), say: "The ask went out, so its rule reads as acted" },
  { later: region("Comments on PR #812"), say: "Comments arrive. Triage marks two as needing a change and one as not worth acting on" },
  { press: button(/^Open in the pull request: .* on src\/cart\/totals\.ts/), say: "A comment opens the pull request at that comment" },
  { look: inside(PR_PANEL, SEND_MAYA), say: "The triage is on the comment itself: a mark for what it needs, where in the code, and its act" },
  { press: inside(PR_PANEL, SEND_MAYA), say: "Send to Job hands the comment to the agent" },
  { look: inside(PR_PANEL, role("img", "Sent to the Job")), say: "A check says it went" },
  { press: back(PR_PANEL), say: "Back to the Workflow" },
  { later: text("Waiting on PR #812: Approved"), say: "Land waits on the PR being approved" },
  { later: node("Deploy: Rolling out"), say: "Merged. The deploy lane starts, rolling out" },
  { look: inside(JOB_LANES, button("Close the ticket", { exact: true })), say: "The deploy is healthy and the tickets are closed. Every step is done, and the Job's row reads done" },
  { type: "skip the deploy watch on this one\n", into: role("textbox", "Message"), say: "Skip the deploy watch, typed to the agent on this Job" },
  { look: inside(JOB_LANES, role("img", /^Changed on this Job/)), say: "The Rolling out state is marked as changed on this Job, with a dashed frame" },

  { key: "Escape", on: JOB_LANES, say: "Esc shuts Workflow" },
  { press: deckRow("Fix cart rounding"), say: "A second Job, with its deploy still rolling out" },
  { press: inside(role("region", /^Job: /), button("More")), say: "End job is behind the three dots" },
  { press: role("menuitem", "End job"), say: "End lists what is in flight first" },
  { look: dialog("In flight"), say: "Deploy is rolling out and ARM-52 is in review. Ending the Job ends them" },
  { press: inside(dialog("In flight"), button("Keep going", { exact: true })), say: "Keep going" },

  { press: deckRow("Go through app feedback"), say: "A third Job goes through app feedback, on the feature workflow as it started" },
  { press: inside(views, workflowView), say: "Workflow opens: in the side panel under 1800px, in the spine from it" },
  { later: JOB_LANES, say: "The workflow as the Job started it: Propose, Discuss, Approve plan, Walk, Refine, Deliver" },
  { look: inside(JOB_LANES, role("img", "Changed on this Job: Added after Dispatch")), say: "The agent changed its own copy: Dispatch and Oversee added after Approve plan, each marked with a dashed frame, as the deploy watch was" },
  { hover: node("Step: Refine"), say: "Walk and Refine are skipped on this Job only, struck through and marked. The workflow itself is unchanged" },
  { later: node("Step: Oversee"), say: "Dispatch starts a Job for each fix, and Oversee, after it, waits for them" },
  { look: node("Fix cart rounding: PR open, ticket ARM-52, In review"), say: "Two Jobs started. Each is a card with the Drone on a lane of its own, linked to Dispatch" },
  { look: inside(JOB_LANES, text('Waiting on Job "Fix cart rounding", Job "Tidy the PR panel landing": Landed')), say: "Oversee ends when every Job it started has landed, and names them" },
  { key: "Escape", on: JOB_LANES, say: "Esc shuts Workflow" },
  { later: inside(views, workflowView), say: "Both Jobs are out. From here the window plays wider than 1800px" },
  { press: inside(views, workflowView), say: "Wider than 1800px, Workflow opens in the spine column, under the views as Artifacts and Checks do" },
  { look: inside(SPINE_LANES, text("Waiting on Tidy the PR panel landing: Landed")), say: "The column is as wide as the pull request panel while Workflow is open. Fix cart rounding has landed, and Oversee waits on the other" },
  { later: inSpine("Tidy the PR panel landing: PR open"), say: "Tidy the PR panel landing is in review" },
  { look: inSpine("Tidy the PR panel landing: Landed"), say: "Both landed. Oversee has what it waited for, and the two Jobs' Landed nodes are in view" },
  { key: "Escape", on: inSpine("Tidy the PR panel landing: Landed"), say: "Esc shuts it, and the column goes back to its width" },

  ...toSystemPage("Workflows", "Workflows, from System"),
  { press: inside(YOURS, button("dispatch", { exact: true })), say: "dispatch, which the Job proposed, is listed under Yours" },
  { look: inside(LANES, button("Step: Oversee", { exact: true })), say: "Oversee ends on a Job landing, so the Job lifecycle is a lane of its own under the workflow, and Dispatch starts a Job" },
  { look: inside(PROPOSED, text("Oversee")), say: "The change waits on Approve: Assess, Plan, Dispatch, Oversee and Summarise, added to Yours" },
]);

export { workspacesLanes as "workspaces-lanes" };
