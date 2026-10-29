// The Workspace `my-store` as the mock holds it, and the Jobs that run in it: the lifecycles, the rules, the `feature` workflow the lanes are drawn
// under, and a Job's lanes at each moment of the walk `workspaces-lanes`. **A mock's model**: no Fleet serves a Workspace, a lifecycle or a rule yet
// (`docs/design`, Workspaces, lifecycles and rules). Only `scenarios/workspaces-lanes.ts` reads this.

import type { CommentRead, Instance, JobLanes, Lifecycle, Rule, SessionLanes, TriagedComment, Workspace } from "@armada/components";

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

const withInstructions = (id: string, instructions: string): Written[] => STEPS.map((one) => (one.id === id ? { ...one, instructions } : one));

/** `feature` as Armada ships it, the store-web repository's copy that adds its test command to Build, and one of Yours that names the repository each ticket takes in Plan. */
export const STORE_DEFINITIONS: readonly { source: "armada" | "repository" | "kit"; definition: Definition }[] = [
  { source: "armada", definition: define(STEPS) },
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
};

// ---- The lifecycles -------------------------------------------------------------------------------------------------------

const TICKET: Lifecycle = {
  id: "ticket",
  label: "Ticket",
  from: "Jira",
  states: [
    { id: "todo", label: "To do", col: 0, reads: [{ source: "Jira", values: ["To Do", "Backlog"], scope: "workspace" }] },
    { id: "in_progress", label: "In progress", col: 1, reads: [{ source: "Jira", values: ["In Progress"], scope: "workspace" }] },
    {
      id: "in_review",
      label: "In review",
      col: 3,
      reads: [
        { source: "Jira", values: ["Code Review", "In Review"], scope: "workspace" },
        { source: "Jira", values: ["Peer Review"], scope: API },
      ],
    },
    { id: "done", label: "Done", col: 8, end: true, reads: [{ source: "Jira", values: ["Done", "Closed"], scope: "workspace" }] },
  ],
  next: [
    ["todo", "in_progress"],
    ["in_progress", "in_review"],
    ["in_review", "done"],
  ],
};

const PULL_REQUEST: Lifecycle = {
  id: "pull_request",
  label: "PR",
  from: "GitHub",
  states: [
    { id: "draft", label: "Draft", col: 2, reads: [{ source: "GitHub", values: ["Draft"], scope: "workspace" }] },
    { id: "in_review", label: "In review", col: 3, reads: [{ source: "GitHub", values: ["Ready for review", "Review requested"], scope: "workspace" }] },
    { id: "approved", label: "Approved", col: 5, reads: [{ source: "GitHub", values: ["Approved by a reviewer"], scope: "workspace" }] },
    { id: "merged", label: "Merged", col: 6, end: true, reads: [{ source: "GitHub", values: ["Merged"], scope: "workspace" }] },
  ],
  next: [
    ["draft", "in_review"],
    ["in_review", "approved"],
    ["approved", "merged"],
  ],
};

const DEPLOY: Lifecycle = {
  id: "deploy",
  label: "Deploy",
  from: "Rollout watch",
  states: [
    { id: "rolling_out", label: "Rolling out", col: 7, reads: [{ source: "Rollout watch", values: ["rolling out"], scope: "workspace" }] },
    { id: "healthy", label: "Healthy", col: 8, end: true, reads: [{ source: "Rollout watch", values: ["healthy"], scope: "workspace" }] },
  ],
  next: [["rolling_out", "healthy"]],
};

const LIFECYCLES = [TICKET, PULL_REQUEST, DEPLOY] as const;

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
};

const ASK: Rule = {
  id: "ask",
  label: "Ask for review",
  from: { lane: "workflow", item: "open_pr" },
  at: "ends",
  to: { lane: "pull_request", item: "in_review" },
  ifs: [],
  does: [
    { act: "message", text: "Draft the review ask to the reviewers" },
    { act: "forge", text: "Request the reviewers on the pull request" },
  ],
  autonomy: "asks",
  kept: "armada",
  scope: "workspace",
  labelAt: 0.25,
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
  labelAt: 0.75,
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
  labelAt: 0.4,
};

export const MY_STORE: Workspace = {
  id: "my-store",
  name: "my-store",
  repositories: [
    { id: WEB, name: WEB },
    { id: API, name: API },
  ],
  steps: {
    plan: { col: 0 },
    build: { col: 1 },
    open_pr: { col: 2 },
    get_it_ready: { col: 3 },
    answer_reviews: { col: 4 },
    land: { col: 5, endsOn: { lane: "pull_request", state: "approved" } },
  },
  lifecycles: LIFECYCLES,
  rules: [PROGRESS, REVIEW, ASK, TRIAGE, DEPLOY_WATCH, CLOSE_ON_MERGE],
};

/** A change typed to the agent that is about rules, and the rule it comes back with. Matched on the lower-cased words. */
export const PROPOSALS: readonly { asked: string; rule: Rule }[] = [{ asked: "close the ticket", rule: CLOSE }];

// ---- A Job in it ----------------------------------------------------------------------------------------------------------

const COPY_STEPS = STEPS.map((one, at) => ({ id: one.id, label: one.label, col: at, ...(one.id === "land" ? { endsOn: { lane: "pull_request" as const, state: "approved" } } : {}) }));
const COPY_RULES = [...MY_STORE.rules, CLOSE];

const instance = (id: string, lifecycle: Instance["lifecycle"], label: string, state: string, visited: readonly string[], rest: Partial<Instance> = {}): Instance => ({ id, lifecycle, label, state, visited, ...rest });

const COMMENTS: readonly { id: string; author: string; where: string; text: string; read: CommentRead }[] = [
  { id: "cm-1", author: "Maya", where: "src/cart/totals.ts:42", text: "This rounds before the discount is applied, so a total can be a cent off.", read: "change" },
  { id: "cm-2", author: "Jun", where: "api/orders.ts:118", text: "Rename giftCardId to cardId to match the rest of the file.", read: "change" },
  { id: "cm-3", author: "Maya", where: "README.md:9", text: "Nit: trailing space.", read: "skip" },
];

/** The marker a comment carries once it has been sent to the Job. */
export const triaged = (sent: readonly string[]): TriagedComment[] => COMMENTS.map((one) => ({ ...one, ...(sent.includes(one.id) ? { sent: true as const } : {}) }));

/**
 * Job A, `ARM-41` and `ARM-42`, at a moment of the walk: 0 dispatched, 1 Plan starts, 2 the PR opens, 3 the review ask waits, 4 comments are triaged,
 * 5 Land waits on the approval, 6 merged and rolling out, 7 healthy and the tickets closed, 8 the deploy watch skipped.
 */
export function jobAt(moment: number, sent: readonly string[] = []): SessionLanes {
  const at = [-1, 0, 2, 3, 4, 5, 6, 6, 6][moment]!;
  const ticket = moment >= 7 ? "done" : moment >= 2 ? "in_review" : moment >= 1 ? "in_progress" : "todo";
  const walked = ["todo", "in_progress", "in_review", "done"];
  const seen = walked.slice(0, walked.indexOf(ticket) + 1);
  const assignee = moment >= 1 ? { assignee: "you" } : {};
  const pr = moment >= 6 ? "merged" : moment >= 3 ? "in_review" : "draft";
  const prSeen = moment >= 6 ? ["draft", "in_review", "approved", "merged"] : moment >= 3 ? ["draft", "in_review"] : ["draft"];
  const instances: Instance[] = [
    instance("arm-41", "ticket", "ARM-41", ticket, seen, { title: "Add gift cards to the cart", ...assignee }),
    instance("arm-42", "ticket", "ARM-42", ticket, seen, { title: "Gift card balance endpoint", ...assignee }),
    ...(moment >= 2 ? [instance("pr-812", "pull_request", "PR #812", pr, prSeen)] : []),
    ...(moment >= 6 ? [instance("deploy", "deploy", "Deploy", moment >= 7 ? "healthy" : "rolling_out", moment >= 7 ? ["rolling_out", "healthy"] : ["rolling_out"])] : []),
  ];
  const fired = [
    ...(moment >= 1 ? ["arm-41", "arm-42"].map((one) => ({ rule: "progress", instance: one, outcome: "acted" as const, at: "12:04" })) : []),
    ...(moment >= 2 ? ["arm-41", "arm-42"].map((one) => ({ rule: "review", instance: one, outcome: "acted" as const, at: "12:31" })) : []),
    ...(moment >= 3 ? [{ rule: "ask", instance: "pr-812", outcome: "asked" as const, at: "12:40" }] : []),
    ...(moment >= 4 ? [{ rule: "triage", instance: "pr-812", outcome: "acted" as const, at: "13:02" }] : []),
    ...(moment >= 6 ? [{ rule: "deploy", instance: "deploy", outcome: "acted" as const, at: "13:40" }] : []),
    ...(moment >= 7 ? ["arm-41", "arm-42"].map((one) => ({ rule: "close", instance: one, outcome: "acted" as const, at: "13:52" })) : []),
  ];
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: LIFECYCLES, rules: COPY_RULES },
    instances,
    fired,
    changes: moment >= 8 ? [{ id: "skip-deploy-watch", lane: "deploy", text: "Skip the deploy watch", at: "14:05" }] : [],
    ...(moment === 5 ? { waitingOn: { step: "land", instance: "pr-812", state: "approved" } } : {}),
    ...(moment >= 4 && moment <= 5 ? { triage: { instance: "pr-812", comments: triaged(sent) } } : {}),
  };
  return { workflow: "feature", at, job };
}

/** Job B, ended while its deploy rolls out: its pull request is merged, one ticket is still in review. */
export function jobB(): SessionLanes {
  const job: JobLanes = {
    workspace: MY_STORE.id,
    copy: { steps: COPY_STEPS, lifecycles: LIFECYCLES, rules: COPY_RULES },
    instances: [
      instance("arm-52", "ticket", "ARM-52", "in_review", ["todo", "in_progress", "in_review"], { title: "Cart totals are a cent off", assignee: "you" }),
      instance("pr-815", "pull_request", "PR #815", "merged", ["draft", "in_review", "approved", "merged"]),
      instance("deploy-815", "deploy", "Deploy", "rolling_out", ["rolling_out"]),
    ],
    fired: [
      { rule: "progress", instance: "arm-52", outcome: "acted", at: "09:12" },
      { rule: "review", instance: "arm-52", outcome: "acted", at: "09:48" },
      { rule: "deploy", instance: "deploy-815", outcome: "acted", at: "10:20" },
    ],
    changes: [],
  };
  return { workflow: "feature", at: 6, job };
}
