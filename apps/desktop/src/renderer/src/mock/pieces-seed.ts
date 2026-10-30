// The Workspaces, lifecycles and rules a mock Fleet keeps, as the reads answer them. A scenario hands its own (`draft.pieces`); saving and deleting change them for the life of the window
// (`slices/lifecycles.ts`). The pieces are the wire's, so the Workflows page reads them as it reads a real Fleet's and draws what they say.

import type { AppliedLifecycle, AppliedRule, LifecycleDefinition, LifecycleRule, WorkspaceOfRepositories } from "@armada/protocol";

/** Every piece the mock Fleet keeps, each with where it is kept. A read answers those that apply to the repository asked about. */
export type PiecesSeed = {
  workspaces: readonly WorkspaceOfRepositories[];
  lifecycles: readonly AppliedLifecycle[];
  rules: readonly AppliedRule[];
};

/** A repository of the seed: the id its pieces are read under and the root its folder is in. */
type Repository = { id: string; root: string };

const EVERYWHERE = { kind: "everywhere" } as const;

const state = (id: string, label: string, read: LifecycleDefinition["states"][number]["read"]) => ({ id, label, read });

const TICKET: LifecycleDefinition = {
  id: "ticket",
  name: "Ticket",
  scope: EVERYWHERE,
  states: [
    state("todo", "To do", { source: "tracker", words: ["To Do", "Backlog"] }),
    state("in_progress", "In progress", { source: "tracker", words: ["In Progress"] }),
    state("in_review", "In review", { source: "tracker", words: ["In Review"] }),
    state("done", "Done", { source: "tracker", words: ["Done", "Closed"] }),
  ],
  transitions: [
    { from: "todo", to: "in_progress" },
    { from: "in_progress", to: "in_review" },
    { from: "in_review", to: "done" },
  ],
  ends: ["done"],
};

const PULL_REQUEST: LifecycleDefinition = {
  id: "pull_request",
  name: "Pull request",
  scope: EVERYWHERE,
  states: [
    state("draft", "Draft", { source: "forge", fact: "draft" }),
    state("ready", "Ready", { source: "forge", fact: "ready" }),
    state("in_review", "In review", { source: "forge", fact: "in_review" }),
    state("approved", "Approved", { source: "forge", fact: "approved" }),
    state("merged", "Merged", { source: "forge", fact: "merged" }),
  ],
  transitions: [
    { from: "draft", to: "ready" },
    { from: "ready", to: "in_review" },
    { from: "in_review", to: "approved" },
    { from: "approved", to: "merged" },
  ],
  ends: ["merged"],
};

const DEPLOY: LifecycleDefinition = {
  id: "deploy",
  name: "Deploy",
  scope: EVERYWHERE,
  states: [state("rolling_out", "Rolling out", { source: "watch", state: "running" }), state("healthy", "Healthy", { source: "watch", state: "passed" })],
  transitions: [{ from: "rolling_out", to: "healthy" }],
  ends: ["healthy"],
};

const held = (lifecycle: LifecycleDefinition): AppliedLifecycle => ({ lifecycle, kept: { rank: { reach: "everywhere", source: "armada" } } });

const rule = (id: string, name: string, when: LifecycleRule["when"], does: LifecycleRule["does"], more: Partial<LifecycleRule> = {}): LifecycleRule => ({
  id,
  name,
  scope: EVERYWHERE,
  when,
  conditions: [],
  does,
  on_failure: { retries: 2, then: "ask_owner" },
  ...more,
});

/**
 * The Workspace `my-store`, whose home is the first repository: Armada's Ticket, Pull request and Deploy for everywhere; the first repository's own Ticket, which it names In review as In QA; a rule
 * of the person's own that starts the ticket, one the repository committed that closes it on merge, and one for the Workspace alone that brings the pull request to Ready.
 */
export function storePieces(web: Repository, api: Repository): PiecesSeed {
  const workspace = "my-store";
  return {
    workspaces: [
      { id: workspace, name: workspace, repositories: [web.id, api.id], home: web.id, source: "repository", path: `${web.root}/.armada/workspaces/${workspace}.toml` },
    ],
    lifecycles: [
      held(TICKET),
      held(PULL_REQUEST),
      held(DEPLOY),
      {
        lifecycle: { ...TICKET, scope: { kind: "within", id: web.id }, states: TICKET.states.map((one) => (one.id === "in_review" ? { ...one, label: "In QA", read: { source: "tracker" as const, words: ["QA"] } } : one)) },
        kept: { rank: { reach: "repository", source: "repository" }, path: `${web.root}/.armada/lifecycles/ticket.toml` },
      },
    ],
    rules: [
      {
        rule: rule(
          "progress",
          "Start the ticket",
          { kind: "enters", lifecycle: "pull_request", state: "draft" },
          [{ kind: "tracker_write", autonomy: "acts", what: { act: "tracker_move", to: "in_progress", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
          { autonomy: "acts" },
        ),
        kept: { rank: { reach: "everywhere", source: "kit" }, path: "/Users/user/.armada/rules/progress.toml" },
      },
      {
        rule: rule(
          "close-on-merge",
          "Close the ticket",
          { kind: "enters", lifecycle: "pull_request", state: "merged" },
          [{ kind: "tracker_write", autonomy: "asks", what: { act: "tracker_close", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
          { autonomy: "asks" },
        ),
        kept: { rank: { reach: "everywhere", source: "repository" }, path: `${web.root}/.armada/rules/close-on-merge.toml` },
      },
      {
        rule: rule(
          "ready",
          "Mark ready",
          { kind: "enters", lifecycle: "ticket", state: "in_review" },
          [{ kind: "forge_write", autonomy: "asks", what: { act: "forge_mark_ready", target: { among: { kind: "held", lifecycle: "pull_request" } } } }],
          { scope: { kind: "within", id: workspace }, autonomy: "asks" },
        ),
        kept: { rank: { reach: "workspace", source: "kit" }, path: "/Users/user/.armada/rules/ready.toml" },
      },
    ],
  };
}
