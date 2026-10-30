// The Workspace `my-store` as the mock holds it, and the Jobs that run in it: the lifecycles, the rules, the `feature` and `planning` workflows the lanes are drawn
// under, and a Job's lanes at each moment of the walk `workspaces-lanes`. **The page's own mock model**, layered as the editor is and with the placement and
// the rows Fleet does not carry (`docs/design`, Workspaces, lifecycles and rules). `workspaces-pieces` is the same page over pieces a mock Fleet serves. Only `scenarios/workspaces-lanes.ts` reads this.

import { lifecyclesFor } from "@armada/components";
import type { CommentRead, Fired, Instance, JobChange, JobLanes, LaneStep, LifecycleDef, Observed, Rule, SessionLanes, TriagedComment, Workspace } from "@armada/components";

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

/** The dispatch workflow a Job proposes: it splits what came in into pieces, starts a Job for each, and waits for every one to land. */
const DISPATCH_STEPS: Written[] = [
  { id: "assess", label: "Assess", instructions: "Read what came in and group it into pieces of work. Name each piece and say which are left out." },
  { id: "plan", label: "Plan", instructions: "Write what each piece changes. Wait for the person to approve it." },
  { id: "dispatch", label: "Dispatch", instructions: "Start a Job for each piece, with its part of the plan as the brief." },
  { id: "oversee", label: "Oversee", instructions: "Wait for every Job started to land. Answer what any of them asks." },
  { id: "summarise", label: "Summarise", instructions: "Say what landed and what was left out." },
];

const DISPATCH_FOR = "a pile of separate fixes that each need their own Job";

/** The text of the dispatch workflow, as the Job offers it. */
export const dispatchText = (): string => JSON.stringify({ version: 1, workflow_id: "dispatch", name: "dispatch", for_requests: DISPATCH_FOR, steps: DISPATCH_STEPS }, null, 2);

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
  assess: { ends: "every piece is named" },
  dispatch: { ends: "every piece has a Job" },
  oversee: { ends: "every Job it started has landed" },
  summarise: { ends: "the summary is posted" },
};

// ---- The lifecycles -------------------------------------------------------------------------------------------------------

// The words a state is read as are set at a level: `default`, the Workspace's (`workspace`) or one repository's, by its id. In review is In Review by default, Code Review and In Review for my-store
// and Peer Review in store-api, so each tab reads it differently and the two below the default carry the changed mark.
const TICKET: LifecycleDef = {
  id: "ticket",
  label: "Ticket",
  from: "Jira",
  kind: "tracker",
  states: [
    { id: "todo", label: "To do", reads: [{ source: "Jira", values: ["To Do", "Backlog"], scope: "default" }] },
    { id: "ready", label: "Ready", reads: [{ source: "Jira", values: ["Ready for Dev", "Selected for Development"], scope: "default" }] },
    { id: "in_progress", label: "In progress", reads: [{ source: "Jira", values: ["In Progress"], scope: "default" }] },
    {
      id: "in_review",
      label: "In review",
      reads: [
        { source: "Jira", values: ["In Review"], scope: "default" },
        { source: "Jira", values: ["Code Review", "In Review"], scope: "workspace" },
        { source: "Jira", values: ["Peer Review"], scope: API },
      ],
    },
    { id: "done", label: "Done", end: true, reads: [{ source: "Jira", values: ["Done", "Closed"], scope: "default" }] },
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
  kind: "forge",
  states: [
    { id: "draft", label: "Draft", reads: [{ source: "GitHub", values: ["Draft"], scope: "default" }] },
    { id: "reviewers_on", label: "Reviewers on", reads: [{ source: "GitHub", values: ["Review requested"], scope: "default" }] },
    { id: "in_review", label: "In review", reads: [{ source: "GitHub", values: ["Ready for review", "Review comments"], scope: "default" }] },
    { id: "approved", label: "Approved", reads: [{ source: "GitHub", values: ["Approved by a reviewer"], scope: "default" }] },
    { id: "merged", label: "Merged", end: true, reads: [{ source: "GitHub", values: ["Merged"], scope: "default" }] },
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
  kind: "watch",
  states: [
    { id: "rolling_out", label: "Rolling out", reads: [{ source: "Rollout watch", values: ["rolling out"], scope: "default" }] },
    { id: "healthy", label: "Healthy", end: true, reads: [{ source: "Rollout watch", values: ["healthy"], scope: "default" }] },
  ],
  next: [["rolling_out", "healthy"]],
};

/** The document a planning Job writes, read from the Job's own Armada document: its draft, the review of it, and the approval. */
const DOCUMENT: LifecycleDef = {
  id: "document",
  label: "Document",
  from: "Armada document",
  kind: "document",
  states: [
    { id: "draft", label: "Draft", reads: [{ source: "Armada document", values: ["draft"], scope: "default" }] },
    { id: "in_review", label: "In review", reads: [{ source: "Armada document", values: ["in review"], scope: "default" }] },
    { id: "approved", label: "Approved", end: true, reads: [{ source: "Armada document", values: ["approved"], scope: "default" }] },
  ],
  next: [
    ["draft", "in_review"],
    ["in_review", "approved"],
  ],
};

/** A Job another Job started, read from Armada itself: it is running, its pull request is open, it has landed. */
const JOB: LifecycleDef = {
  id: "job",
  label: "Job",
  from: "Fleet",
  kind: "job",
  states: [
    { id: "started", label: "Started", reads: [{ source: "Fleet", values: ["running"], scope: "default" }] },
    { id: "pr_open", label: "PR open", reads: [{ source: "Fleet", values: ["pull request open"], scope: "default" }] },
    { id: "landed", label: "Landed", end: true, reads: [{ source: "Fleet", values: ["merged into main"], scope: "default" }] },
  ],
  next: [
    ["started", "pr_open"],
    ["pr_open", "landed"],
  ],
};

const LIFECYCLES = [DOCUMENT, TICKET, PULL_REQUEST, DEPLOY, JOB] as const;

/**
 * What a step can wait on besides a lifecycle, each as the dropdowns list it: the thing, then its state. `says` is the sentence `STORE_ENDS` ends a step with, so each step opens with its own
 * pair chosen. The pull request, the document and the Jobs this Job starts are lifecycles too, and name the state a step that waits on them is held on.
 */
const OBSERVES: readonly Observed[] = [
  { id: "agent", label: "The agent", states: [{ id: "done", label: "says it is done", says: "the agent says it is done" }] },
  { id: "you", label: "You", states: [{ id: "approve_plan", label: "approve the plan", says: "you approve the plan" }] },
  { id: "change", label: "The change", states: [{ id: "pushed", label: "is pushed", says: "the change is pushed" }] },
  {
    id: "pull_request",
    label: "The pull request",
    lifecycle: "pull_request",
    states: [
      { id: "open", label: "is open", says: "the pull request is open" },
      { id: "checks", label: "has green checks", says: "the pull request has green checks" },
      { id: "ready", label: "is ready for review", says: "the pull request is ready for review" },
      { id: "approved", label: "is approved", says: "the pull request is approved", endsOn: { lane: "pull_request", state: "approved" } },
      { id: "landed", label: "has landed", says: "the pull request has landed", endsOn: { lane: "pull_request", state: "merged" } },
    ],
  },
  { id: "comments", label: "Every review comment", states: [{ id: "answered", label: "has an answer", says: "every comment has an answer" }] },
  { id: "questions", label: "Every question", states: [{ id: "answered", label: "is answered", says: "every question is answered" }] },
  {
    id: "document",
    label: "The document",
    lifecycle: "document",
    states: [
      { id: "draft", label: "is a draft", says: "the document is a draft", endsOn: { lane: "document", state: "draft" } },
      { id: "in_review", label: "is in review", says: "the document is in review", endsOn: { lane: "document", state: "in_review" } },
      { id: "approved", label: "is approved", says: "the document is approved", endsOn: { lane: "document", state: "approved" } },
    ],
  },
  { id: "tickets", label: "The tickets", states: [{ id: "linked", label: "link the document", says: "each ticket links the document" }] },
  {
    id: "pieces",
    label: "Every piece",
    states: [
      { id: "named", label: "is named", says: "every piece is named" },
      { id: "has_job", label: "has a Job", says: "every piece has a Job" },
    ],
  },
  { id: "summary", label: "The summary", states: [{ id: "posted", label: "is posted", says: "the summary is posted" }] },
  {
    id: "jobs",
    label: "The Jobs this Job starts",
    lifecycle: "job",
    states: [
      { id: "all_landed", label: "every one landed", says: "every Job it started has landed", endsOn: { lane: "job", state: "landed" } },
      { id: "any_landed", label: "any one landed", says: "any Job it started has landed", endsOn: { lane: "job", state: "landed" } },
    ],
  },
];

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
  scope: "default",
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
  scope: "default",
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
  scope: "default",
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
  scope: "default",
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
  scope: "default",
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
  scope: "default",
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
  scope: "default",
  workflow: "planning",
};

/** The dispatch workflow's rule: Dispatch starting starts a Job for each piece, and each Job is a lane of its own. */
export const START_JOB: Rule = {
  id: "start_job",
  label: "Start a Job",
  from: { lane: "workflow", item: "dispatch" },
  at: "starts",
  to: { lane: "job", item: "started" },
  ifs: [],
  does: [{ act: "job", text: "Start a Job for each piece" }],
  autonomy: "acts",
  kept: "armada",
  scope: "default",
  workflow: "dispatch",
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
  scope: "default",
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
  observes: OBSERVES,
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
    dispatch: {
      steps: {
        assess: { col: 0 },
        plan: { col: 1 },
        dispatch: { col: 2 },
        oversee: { col: 3, endsOn: { lane: "job", state: "landed" } },
        summarise: { col: 5 },
      },
      states: {
        "job/started": 2,
        "job/pr_open": 3,
        "job/landed": 4,
      },
    },
  },
  rules: [PROGRESS, REVIEW, ASK, TRIAGE, DEPLOY_WATCH, CLOSE_ON_MERGE, DOCUMENT_REVIEW, READY, START_JOB],
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

export const COMMENTS: readonly { id: string; author: string; where: string; text: string; read: CommentRead }[] = [
  { id: "cm-1", author: "Maya", where: "src/cart/totals.ts:42", text: "This rounds before the discount is applied, so a total can be a cent off.", read: "change" },
  { id: "cm-2", author: "Jun", where: "api/orders.ts:118", text: "Rename giftCardId to cardId to match the rest of the file.", read: "change" },
  { id: "cm-3", author: "Maya", where: "README.md:9", text: "Nit: trailing space.", read: "skip" },
];

/** The marker a comment carries once it has been sent to the Job. */
export const triaged = (sent: readonly string[]): TriagedComment[] => COMMENTS.map((one) => ({ ...one, ...(sent.includes(one.id) ? { sent: true as const } : {}) }));

/** The id of the draft the review ask is, which the rule waits on: the card in the thread and the item the rail carries are the same one. */
export const ASK_DRAFT = "ra-1";

/** What the agent is doing at the moments it works, in its own account: the line the Now beat reads, and what it has done. The moments it waits on the person it does nothing. */
export const WORKING: Readonly<Record<number, { now: string; done: readonly string[] }>> = {
  0: { now: "Reading ARM-41 and ARM-42", done: [] },
  1: { now: "Planning the change in store-web and store-api", done: [] },
  2: { now: "Opening the pull request for ARM-41 and ARM-42", done: ["Planned the change in store-web and store-api", "Built the change in both repositories"] },
  4: { now: "Reading the review comments", done: ["Opened the pull request", "Put Maya and Jun on it as reviewers"] },
  6: { now: "Watching the rollout of PR #812", done: ["Merged the pull request", "Started the rollout watch"] },
};

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
    ...(moment >= 1 ? ["arm-41", "arm-42"].map((one) => ({ rule: "progress", instance: one, outcome: "acted" as const, at: "12:04", did: ["Assigned to you, moved to In progress"] })) : []),
    ...(moment >= 2 ? ["arm-41", "arm-42"].map((one) => ({ rule: "review", instance: one, outcome: "acted" as const, at: "12:31", did: ["Moved to In review"] })) : []),
    ...(moment >= 3
      ? [{ rule: "ask", instance: "pr-812", outcome: asked ? ("acted" as const) : ("asked" as const), at: asked ? "12:44" : "12:40", did: [asked ? "Review ask sent" : "Review ask drafted"], ...(asked ? {} : { needs: ASK_DRAFT }) }]
      : []),
    ...(moment >= 4 ? [{ rule: "triage", instance: "pr-812", outcome: "acted" as const, at: "13:02", did: ["Comments read"] }] : []),
    ...(moment >= 6 ? [{ rule: "deploy", instance: "deploy", outcome: "acted" as const, at: "13:40", did: ["Started the rollout watch"] }] : []),
    ...(moment >= 7 ? ["arm-41", "arm-42"].map((one) => ({ rule: "close", instance: one, outcome: "acted" as const, at: "13:52", did: ["Moved to Done"] })) : []),
  ];
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: COPY_LIFECYCLES, rules: COPY_RULES },
    instances,
    fired,
    changes: moment >= 8 ? [{ id: "skip-deploy-watch", lane: "deploy", item: "rolling_out", text: "Skip the deploy watch", at: "14:05" }] : [],
    ...(moment === 5 ? { waiting: ["Waiting on PR #812: Approved"] } : {}),
    ...(moment >= 4 && moment <= 5 ? { triage: { instance: "pr-812", comments: triaged(sent) } } : {}),
  };
  return { workflow: "feature", at, working: WORKING[moment] !== undefined, job };
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
      { rule: "progress", instance: "arm-52", outcome: "acted", at: "09:12", did: ["Assigned to you, moved to In progress"] },
      { rule: "review", instance: "arm-52", outcome: "acted", at: "09:48", did: ["Moved to In review"] },
      { rule: "deploy", instance: "deploy-815", outcome: "acted", at: "10:20", did: ["Started the rollout watch"] },
    ],
    changes: [],
  };
  return { workflow: "feature", at: 6, working: false, job };
}


// ---- The feedback Job -----------------------------------------------------------------------------------------------------

/** The id of the Job that goes through the app feedback, and of the two Jobs it starts. */
export const FEEDBACK = "app-feedback";
export const CART = "job-cart";
const PANEL = "job-pr-panel";

/** The workflow the Job started on, and its steps as the rail names them. */
export const FEEDBACK_STEPS = ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"];
/** The same after the Job changed its own copy: Walk and Refine skipped, Dispatch and Oversee after Approve plan. */
export const FEEDBACK_STEPS_CHANGED = ["Propose", "Discuss", "Approve plan", "Dispatch", "Oversee", "Walk", "Refine", "Deliver"];
export const FEEDBACK_SKIPPED = [5, 6] as const;

const FEEDBACK_COPY_START: readonly LaneStep[] = [
  { id: "propose", label: "Propose", col: 0 },
  { id: "discuss", label: "Discuss", col: 1 },
  { id: "approve_plan", label: "Approve plan", col: 2 },
  { id: "walk", label: "Walk", col: 3 },
  { id: "refine", label: "Refine", col: 4 },
  { id: "deliver", label: "Deliver", col: 5 },
];

const FEEDBACK_COPY_CHANGED: readonly LaneStep[] = [
  { id: "propose", label: "Propose", col: 0 },
  { id: "discuss", label: "Discuss", col: 1 },
  { id: "approve_plan", label: "Approve plan", col: 2 },
  { id: "dispatch", label: "Dispatch", col: 3 },
  { id: "oversee", label: "Oversee", col: 4, endsOn: { lane: "job", state: "landed" } },
  { id: "walk", label: "Walk", col: 5 },
  { id: "refine", label: "Refine", col: 6 },
  { id: "deliver", label: "Deliver", col: 7 },
];

/** The Job lifecycle as this Job's copy draws it, its states under Dispatch and Oversee. */
const JOB_COPY = { ...JOB, states: JOB.states.map((state, at) => ({ ...state, col: 3 + at })) };

const JOB_SEEN: Record<string, readonly string[]> = { started: ["started"], pr_open: ["started", "pr_open"], landed: ["started", "pr_open", "landed"] };

/**
 * The ticket `Fix cart rounding` was started for. That Job holds it and its rules move it, so the feedback Job has no lane for it and reads it from the Job: the name of the state it
 * stands in follows the Job's. `ARM-58` is one the feedback Job kept, with no Job started for it, so it keeps a lane of its own.
 */
const CART_TICKET = { label: "ARM-52", title: "Cart totals are a cent off" } as const;
const CART_TICKET_STATE: Readonly<Record<string, string>> = { started: "In progress", pr_open: "In review", landed: "Done" };
const TICKET_COPY = COPY_LIFECYCLES.find((one) => one.id === "ticket")!;

/**
 * `Fix cart rounding` as its own lanes read at the moment the feedback Job is at: the ticket the feedback Job reads from it, In review while its pull request is open and Done once it
 * lands, and the pull request. Its workflow is the Workspace's `feature`, at Answer reviews and then done.
 */
export function cartAt(moment: number): SessionLanes {
  const landed = moment >= 3;
  const ticket = landed ? "done" : "in_review";
  const pr = landed ? "merged" : "in_review";
  const walked = ["todo", "in_progress", "in_review", "done"];
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: COPY_LIFECYCLES, rules: [] },
    instances: [
      instance("arm-52", "ticket", CART_TICKET.label, ticket, walked.slice(0, walked.indexOf(ticket) + 1), { title: CART_TICKET.title, assignee: "you" }),
      instance("pr-815", "pull_request", "PR #815", pr, PR_SEEN[pr]!),
    ],
    fired: [],
    changes: [],
  };
  return { workflow: "feature", at: landed ? COPY_STEPS.length : 4, working: false, job };
}

/** What the feedback Job's agent is doing at the moments it works. */
export const FEEDBACK_WORKING: Readonly<Record<number, { now: string; done: readonly string[] }>> = {
  0: { now: "Going through the feedback", done: [] },
  1: { now: "Splitting the feedback into fixes", done: ["Read the feedback", "Changed this Job's workflow"] },
  4: { now: "Proposing dispatch as a workflow", done: ["Started two Jobs", "Both Jobs landed"] },
};

const CHANGES: readonly JobChange[] = [
  { id: "add-dispatch", lane: "workflow", item: "dispatch", text: "Added after Approve plan", at: "14:09" },
  { id: "add-oversee", lane: "workflow", item: "oversee", text: "Added after Dispatch", at: "14:09" },
  { id: "skip-walk", lane: "workflow", item: "walk", text: "Skipped on this Job", at: "14:09" },
  { id: "skip-refine", lane: "workflow", item: "refine", text: "Skipped on this Job", at: "14:09" },
];

/**
 * The Job that goes through the app feedback, at a moment: 0 it reads the feedback on the `feature` workflow as it started, 1 it has changed its own copy (Walk and Refine skipped,
 * Dispatch and Oversee added), 2 it has started two Jobs, one of them for a ticket, and keeps a ticket of its own, and Oversee waits on both Jobs, 3 one has landed, 4 both have landed and Oversee is done.
 */
export function feedbackAt(moment: number): SessionLanes {
  const changed = moment >= 1;
  const cart = moment >= 3 ? "landed" : moment >= 2 ? "pr_open" : "started";
  const panel = moment >= 4 ? "landed" : moment >= 3 ? "pr_open" : "started";
  const instances: Instance[] =
    moment >= 2
      ? [
          instance(CART, "job", "Fix cart rounding", cart, JOB_SEEN[cart]!, { ticket: { ...CART_TICKET, state: CART_TICKET_STATE[cart]! } }),
          instance(PANEL, "job", "Tidy the PR panel landing", panel, JOB_SEEN[panel]!),
          instance("arm-58", "ticket", "ARM-58", "todo", ["todo"], { title: "Empty wishlist link 404s" }),
        ]
      : [];
  const fired: Fired[] = moment >= 2 ? [CART, PANEL].map((one) => ({ rule: "start_job", instance: one, outcome: "acted" as const, at: "14:12", did: ["Started the Job"] })) : [];
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: changed ? { steps: FEEDBACK_COPY_CHANGED, lifecycles: moment >= 2 ? [JOB_COPY, TICKET_COPY] : [JOB_COPY], rules: [START_JOB] } : { steps: FEEDBACK_COPY_START, lifecycles: [], rules: [] },
    instances,
    fired,
    changes: changed ? CHANGES : [],
    ...(moment === 2 ? { waiting: ['Waiting on Job "Fix cart rounding", Job "Tidy the PR panel landing": Landed'] } : {}),
    ...(moment === 3 ? { waiting: ["Waiting on Tidy the PR panel landing: Landed"] } : {}),
  };
  const at = [1, 3, 4, 4, 4][moment]!;
  return { workflow: "feature", at, working: FEEDBACK_WORKING[moment] !== undefined, ...(changed ? { skipped: FEEDBACK_SKIPPED } : {}), job };
}
