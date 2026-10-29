// The Workspace `my-store` as the mock holds it, and the Jobs that run in it: the lifecycles, the rules, the `feature` and `planning` workflows the lanes are drawn
// under, and a Job's lanes at each moment of the walk `workspaces-lanes`. **A mock's model**: no Fleet serves a Workspace, a lifecycle or a rule yet
// (`docs/design`, Workspaces, lifecycles and rules). Only `scenarios/workspaces-lanes.ts` reads this.

import { lifecyclesFor } from "@armada/components";
import type { CommentRead, Fired, Instance, JobLanes, LifecycleDef, Rule, SessionLanes, TriagedComment, Workspace } from "@armada/components";

import type { Definition } from "./workflows-managed";

export const WEB = "store-web";
export const API = "store-api";

// ---- The workflow ---------------------------------------------------------------------------------------------------------

type Written = Definition["steps"][number];

const STEPS: Written[] = [
  { id: "plan", label: "Plan", instructions: "Read the tickets. Write what changes in each repository and what is left out. Wait for the person to approve it." },
  { id: "build", label: "Build", instructions: "Make the change in each repository. Keep to the plan and run the repository's checks before going on." },
  { id: "open_pr", label: "Open PR", instructions: "Push the branch and open the pull request. Name every ticket it is for." },
  { id: "get_it_ready", label: "Get it ready", instructions: "Bring the pull request to ready for review: green checks, a description that says why, reviewers named." },
  { id: "answer_reviews", label: "Answer reviews", instructions: "Read each comment the reviewers leave. Change what needs a change, answer what only needs an answer, and leave the rest." },
  { id: "land", label: "Land", instructions: "Wait for the approval, then merge the pull request on a green ci." },
];

const FOR = "adding or changing something a customer uses in the store";

const define = (steps: Written[]): Definition => ({ version: 1, workflow_id: "feature", name: "feature", for_requests: FOR, steps });

/** The planning workflow: a design document is written and reviewed before any ticket is ready to build. */
const PLANNING_STEPS: Written[] = [
  { id: "read", label: "Read the tickets", instructions: "Read each ticket and the code it touches. Note what is unclear and ask the person before writing." },
  { id: "write", label: "Write the design", instructions: "Write the design as a document: what changes in each repository, what it costs and what is left out." },
  { id: "get_review", label: "Get it reviewed", instructions: "Put the document in front of the reviewers. Answer what they say and change the document to match." },
  { id: "hand_off", label: "Hand off", instructions: "Link the approved document in each ticket and say what builds first." },
];

const PLANNING_FOR = "a change in the store that needs a design before anyone builds it";
const planning = (): Definition => ({ version: 1, workflow_id: "planning", name: "planning", for_requests: PLANNING_FOR, steps: PLANNING_STEPS });

const withInstructions = (id: string, instructions: string): Written[] => STEPS.map((one) => (one.id === id ? { ...one, instructions } : one));

/** `feature` as Armada ships it, the store-web repository's copy that adds its test command to Build, and one of Yours that names the repository each ticket takes in Plan. */
export const STORE_DEFINITIONS: readonly { source: "armada" | "repository" | "kit"; definition: Definition }[] = [
  { source: "armada", definition: define(STEPS) },
  { source: "armada", definition: planning() },
  { source: "repository", definition: define(withInstructions("build", "Make the change in each repository. Keep to the plan and run the repository's checks before going on. Run pnpm test in store-web before pushing.")) },
  { source: "kit", definition: define(withInstructions("plan", "Read the tickets. Write what changes in each repository, which repository takes each ticket, and what is left out. Wait for the person to approve it.")) },
];

export const STORE_ENDS: Readonly<Record<string, { ends: string; endsAny?: readonly string[]; passedOver?: string }>> = {
  plan: { ends: "you approve the plan" },
  build: { ends: "the change is pushed" },
  open_pr: { ends: "the pull request is open" },
  get_it_ready: { ends: "the pull request is ready for review" },
  answer_reviews: { ends: "every comment has an answer" },
  land: { ends: "the pull request is approved" },
  read: { ends: "every question is answered" },
  write: { ends: "the document is a draft" },
  get_review: { ends: "the document is approved" },
  hand_off: { ends: "each ticket links the document" },
};

// ---- The lifecycles -------------------------------------------------------------------------------------------------------

const TICKET: LifecycleDef = {
  id: "ticket",
  label: "Ticket",
  from: "Jira",
  states: [
    { id: "todo", label: "To do", reads: [{ source: "Jira", values: ["To Do", "Backlog"], scope: "workspace" }] },
    { id: "ready", label: "Ready", reads: [{ source: "Jira", values: ["Ready for Dev", "Selected for Development"], scope: "workspace" }] },
    { id: "in_progress", label: "In progress", reads: [{ source: "Jira", values: ["In Progress"], scope: "workspace" }] },
    {
      id: "in_review",
      label: "In review",
      reads: [
        { source: "Jira", values: ["Code Review", "In Review"], scope: "workspace" },
        { source: "Jira", values: ["Peer Review"], scope: API },
      ],
    },
    { id: "done", label: "Done", end: true, reads: [{ source: "Jira", values: ["Done", "Closed"], scope: "workspace" }] },
  ],
  next: [
    ["todo", "ready"],
    ["todo", "in_progress"],
    ["ready", "in_progress"],
    ["in_progress", "in_review"],
    ["in_review", "done"],
  ],
};

const PULL_REQUEST: LifecycleDef = {
  id: "pull_request",
  label: "PR",
  from: "GitHub",
  states: [
    { id: "draft", label: "Draft", reads: [{ source: "GitHub", values: ["Draft"], scope: "workspace" }] },
    { id: "reviewers_on", label: "Reviewers on", reads: [{ source: "GitHub", values: ["Review requested"], scope: "workspace" }] },
    { id: "in_review", label: "In review", reads: [{ source: "GitHub", values: ["Ready for review", "Review comments"], scope: "workspace" }] },
    { id: "approved", label: "Approved", reads: [{ source: "GitHub", values: ["Approved by a reviewer"], scope: "workspace" }] },
    { id: "merged", label: "Merged", end: true, reads: [{ source: "GitHub", values: ["Merged"], scope: "workspace" }] },
  ],
  next: [
    ["draft", "reviewers_on"],
    ["reviewers_on", "in_review"],
    ["in_review", "approved"],
    ["approved", "merged"],
  ],
};

const DEPLOY: LifecycleDef = {
  id: "deploy",
  label: "Deploy",
  from: "Rollout watch",
  states: [
    { id: "rolling_out", label: "Rolling out", reads: [{ source: "Rollout watch", values: ["rolling out"], scope: "workspace" }] },
    { id: "healthy", label: "Healthy", end: true, reads: [{ source: "Rollout watch", values: ["healthy"], scope: "workspace" }] },
  ],
  next: [["rolling_out", "healthy"]],
};

/** The document a planning Job writes, read from the Job's own Armada document: its draft, the review of it, and the approval. */
const DOCUMENT: LifecycleDef = {
  id: "document",
  label: "Document",
  from: "Armada document",
  states: [
    { id: "draft", label: "Draft", reads: [{ source: "Armada document", values: ["draft"], scope: "workspace" }] },
    { id: "in_review", label: "In review", reads: [{ source: "Armada document", values: ["in review"], scope: "workspace" }] },
    { id: "approved", label: "Approved", end: true, reads: [{ source: "Armada document", values: ["approved"], scope: "workspace" }] },
  ],
  next: [
    ["draft", "in_review"],
    ["in_review", "approved"],
  ],
};

const LIFECYCLES = [DOCUMENT, TICKET, PULL_REQUEST, DEPLOY] as const;

// ---- The rules ------------------------------------------------------------------------------------------------------------

const PROGRESS: Rule = {
  id: "progress",
  label: "Move to In progress",
  from: { lane: "workflow", item: "plan" },
  at: "starts",
  to: { lane: "ticket", item: "in_progress" },
  ifs: [],
  does: [
    { act: "tracker", text: "Move the ticket to In progress" },
    { act: "tracker", text: "Assign the ticket to you" },
  ],
  autonomy: "acts",
  kept: "armada",
  scope: "workspace",
};

const REVIEW: Rule = {
  id: "review",
  label: "Move to In review",
  from: { lane: "pull_request", item: "draft" },
  to: { lane: "ticket", item: "in_review" },
  ifs: ["The ticket is linked to the pull request"],
  does: [{ act: "tracker", text: "Move the ticket to In review" }],
  autonomy: "acts",
  kept: "yours",
  scope: "workspace",
  overrides: [{ scope: API, autonomy: "asks" }],
};

const ASK: Rule = {
  id: "ask",
  label: "Ask for review",
  from: { lane: "pull_request", item: "reviewers_on" },
  to: { lane: "workflow", item: "get_it_ready" },
  ifs: [],
  does: [{ act: "message", text: "Draft the review ask to the reviewers" }],
  autonomy: "asks",
  kept: "armada",
  scope: "workspace",
};

const TRIAGE: Rule = {
  id: "triage",
  label: "Triage comments",
  from: { lane: "pull_request", item: "in_review" },
  to: { lane: "workflow", item: "answer_reviews" },
  ifs: [],
  does: [
    { act: "agent", text: "Read each human comment and mark whether it needs a change" },
    { act: "tell", text: "Tell the Job what needs a change" },
  ],
  autonomy: "acts",
  kept: "armada",
  scope: "workspace",
};

const DEPLOY_WATCH: Rule = {
  id: "deploy",
  label: "Watch the deploy",
  from: { lane: "pull_request", item: "merged" },
  to: { lane: "deploy", item: "rolling_out" },
  ifs: [],
  does: [{ act: "watch", text: "Start the rollout watch" }],
  autonomy: "acts",
  kept: "armada",
  scope: "workspace",
};

const CLOSE_ON_MERGE: Rule = {
  id: "close_on_merge",
  label: "Close on merge",
  from: { lane: "pull_request", item: "merged" },
  to: { lane: "ticket", item: "done" },
  ifs: [],
  does: [{ act: "tracker", text: "Move the ticket to Done" }],
  autonomy: "off",
  kept: "repository",
  scope: API,
};

/** The planning workflow's two rules: the document goes in review when the step starts, and the ticket is ready once the document is approved. */
const DOCUMENT_REVIEW: Rule = {
  id: "document_review",
  label: "Put in review",
  from: { lane: "workflow", item: "get_review" },
  at: "starts",
  to: { lane: "document", item: "in_review" },
  ifs: [],
  does: [{ act: "agent", text: "Mark the document In review" }],
  autonomy: "acts",
  kept: "armada",
  scope: "workspace",
  workflow: "planning",
};

const READY: Rule = {
  id: "ready",
  label: "Ready the ticket",
  from: { lane: "document", item: "approved" },
  to: { lane: "ticket", item: "ready" },
  ifs: ["Every ticket is linked in the document"],
  does: [{ act: "tracker", text: "Move the ticket to Ready" }],
  autonomy: "acts",
  kept: "armada",
  scope: "workspace",
  workflow: "planning",
};

/** What "close the ticket once merged and the deploy is healthy" comes back as when it is typed to the agent. */
export const CLOSE: Rule = {
  id: "close",
  label: "Close the ticket",
  from: { lane: "deploy", item: "healthy" },
  to: { lane: "ticket", item: "done" },
  ifs: ["The pull request is merged"],
  does: [{ act: "tracker", text: "Move the ticket to Done" }],
  autonomy: "acts",
  kept: "yours",
  scope: "workspace",
  workflow: "feature",
};

export const MY_STORE: Workspace = {
  id: "my-store",
  name: "my-store",
  repositories: [
    { id: WEB, name: WEB },
    { id: API, name: API },
  ],
  lifecycles: LIFECYCLES,
  workflows: {
    feature: {
      steps: {
        plan: { col: 0 },
        build: { col: 1 },
        open_pr: { col: 2 },
        get_it_ready: { col: 4 },
        answer_reviews: { col: 5 },
        land: { col: 6, endsOn: { lane: "pull_request", state: "approved" } },
      },
      states: {
        "ticket/todo": 0,
        "ticket/in_progress": 1,
        "ticket/in_review": 3,
        "ticket/done": 9,
        "pull_request/draft": 2,
        "pull_request/reviewers_on": 4,
        "pull_request/in_review": 5,
        "pull_request/approved": 6,
        "pull_request/merged": 7,
        "deploy/rolling_out": 8,
        "deploy/healthy": 9,
      },
    },
    planning: {
      steps: {
        read: { col: 0 },
        write: { col: 1 },
        get_review: { col: 2, endsOn: { lane: "document", state: "approved" } },
        hand_off: { col: 4 },
      },
      states: {
        "document/draft": 1,
        "document/in_review": 2,
        "document/approved": 3,
        "ticket/todo": 0,
        "ticket/ready": 4,
      },
    },
  },
  rules: [PROGRESS, REVIEW, ASK, TRIAGE, DEPLOY_WATCH, CLOSE_ON_MERGE, DOCUMENT_REVIEW, READY],
};

/** A change typed to the agent that is about rules, and the rule it comes back with. Matched on the lower-cased words. */
export const PROPOSALS: readonly { asked: string; rule: Rule }[] = [{ asked: "close the ticket", rule: CLOSE }];

// ---- A Job in it ----------------------------------------------------------------------------------------------------------

const FEATURE = MY_STORE.workflows.feature!;
const COPY_STEPS = STEPS.map((one) => ({ id: one.id, label: one.label, col: FEATURE.steps[one.id]!.col, ...(FEATURE.steps[one.id]!.endsOn === undefined ? {} : { endsOn: FEATURE.steps[one.id]!.endsOn }) }));
const COPY_LIFECYCLES = lifecyclesFor(MY_STORE, "feature");
const COPY_RULES = [...MY_STORE.rules.filter((rule) => rule.workflow === undefined), CLOSE];

/** The states a pull request went through to stand in each. */
const PR_SEEN: Record<string, readonly string[]> = {
  draft: ["draft"],
  reviewers_on: ["draft", "reviewers_on"],
  in_review: ["draft", "reviewers_on", "in_review"],
  merged: ["draft", "reviewers_on", "in_review", "approved", "merged"],
};

const instance = (id: string, lifecycle: Instance["lifecycle"], label: string, state: string, visited: readonly string[], rest: Partial<Instance> = {}): Instance => ({ id, lifecycle, label, state, visited, ...rest });

const COMMENTS: readonly { id: string; author: string; where: string; text: string; read: CommentRead }[] = [
  { id: "cm-1", author: "Maya", where: "src/cart/totals.ts:42", text: "This rounds before the discount is applied, so a total can be a cent off.", read: "change" },
  { id: "cm-2", author: "Jun", where: "api/orders.ts:118", text: "Rename giftCardId to cardId to match the rest of the file.", read: "change" },
  { id: "cm-3", author: "Maya", where: "README.md:9", text: "Nit: trailing space.", read: "skip" },
];

/** The marker a comment carries once it has been sent to the Job. */
export const triaged = (sent: readonly string[]): TriagedComment[] => COMMENTS.map((one) => ({ ...one, ...(sent.includes(one.id) ? { sent: true as const } : {}) }));

/**
 * Job A, `ARM-41` and `ARM-42`, at a moment of the walk: 0 dispatched, 1 Plan starts, 2 the PR opens, 3 reviewers are on and the review ask waits, 4 comments are triaged,
 * 5 Land waits on the approval, 6 merged and rolling out, 7 healthy and the tickets closed, 8 the deploy watch skipped. `sent` is what the person has sent: a comment's id, or `ask`.
 * The times are the mock's one clock; the thread's rows and the rail's steps are stamped from it (`scenarios/workspaces-lanes.ts`).
 */
export function jobAt(moment: number, sent: readonly string[] = []): SessionLanes {
  const at = [-1, 0, 2, 3, 4, 5, 6, 6, 6][moment]!;
  const ticket = moment >= 7 ? "done" : moment >= 2 ? "in_review" : moment >= 1 ? "in_progress" : "todo";
  const walked = ["todo", "in_progress", "in_review", "done"];
  const seen = walked.slice(0, walked.indexOf(ticket) + 1);
  const assignee = moment >= 1 ? { assignee: "you" } : {};
  const pr = moment >= 6 ? "merged" : moment >= 4 ? "in_review" : moment >= 3 ? "reviewers_on" : "draft";
  const prSeen = PR_SEEN[pr]!;
  const reviewers = { says: { reviewers_on: "Maya, Jun" } };
  const instances: Instance[] = [
    instance("arm-41", "ticket", "ARM-41", ticket, seen, { title: "Add gift cards to the cart", ...assignee }),
    instance("arm-42", "ticket", "ARM-42", ticket, seen, { title: "Gift card balance endpoint", ...assignee }),
    ...(moment >= 2 ? [instance("pr-812", "pull_request", "PR #812", pr, prSeen, reviewers)] : []),
    ...(moment >= 6 ? [instance("deploy", "deploy", "Deploy", moment >= 7 ? "healthy" : "rolling_out", moment >= 7 ? ["rolling_out", "healthy"] : ["rolling_out"])] : []),
  ];
  const asked = sent.includes("ask");
  const fired: Fired[] = [
    ...(moment >= 1 ? ["arm-41", "arm-42"].map((one) => ({ rule: "progress", instance: one, outcome: "acted" as const, at: "12:04", did: "Assigned to you, moved to In progress" })) : []),
    ...(moment >= 2 ? ["arm-41", "arm-42"].map((one) => ({ rule: "review", instance: one, outcome: "acted" as const, at: "12:31", did: "Moved to In review" })) : []),
    ...(moment >= 3 ? [{ rule: "ask", instance: "pr-812", outcome: asked ? ("acted" as const) : ("asked" as const), at: asked ? "12:44" : "12:40", did: asked ? "Review ask sent" : "Review ask drafted" }] : []),
    ...(moment >= 4 ? [{ rule: "triage", instance: "pr-812", outcome: "acted" as const, at: "13:02", did: "Comments read" }] : []),
    ...(moment >= 6 ? [{ rule: "deploy", instance: "deploy", outcome: "acted" as const, at: "13:40", did: "Started the rollout watch" }] : []),
    ...(moment >= 7 ? ["arm-41", "arm-42"].map((one) => ({ rule: "close", instance: one, outcome: "acted" as const, at: "13:52", did: "Moved to Done" })) : []),
  ];
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: COPY_LIFECYCLES, rules: COPY_RULES },
    instances,
    fired,
    changes: moment >= 8 ? [{ id: "skip-deploy-watch", lane: "deploy", item: "rolling_out", text: "Skip the deploy watch", at: "14:05" }] : [],
    ...(moment === 5 ? { waitingOn: { step: "land", instance: "pr-812", state: "approved" } } : {}),
    ...(moment >= 4 && moment <= 5 ? { triage: { instance: "pr-812", comments: triaged(sent) } } : {}),
  };
  return { workflow: "feature", at, job };
}

/** Job B, ended while its deploy rolls out: its pull request is merged, one ticket is still in review. */
export function jobB(): SessionLanes {
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: COPY_LIFECYCLES, rules: COPY_RULES },
    instances: [
      instance("arm-52", "ticket", "ARM-52", "in_review", ["todo", "in_progress", "in_review"], { title: "Cart totals are a cent off", assignee: "you" }),
      instance("pr-815", "pull_request", "PR #815", "merged", PR_SEEN.merged!, { says: { reviewers_on: "Maya" } }),
      instance("deploy-815", "deploy", "Deploy", "rolling_out", ["rolling_out"]),
    ],
    fired: [
      { rule: "progress", instance: "arm-52", outcome: "acted", at: "09:12", did: "Assigned to you, moved to In progress" },
      { rule: "review", instance: "arm-52", outcome: "acted", at: "09:48", did: "Moved to In review" },
      { rule: "deploy", instance: "deploy-815", outcome: "acted", at: "10:20", did: "Started the rollout watch" },
    ],
    changes: [],
  };
  return { workflow: "feature", at: 6, job };
}
