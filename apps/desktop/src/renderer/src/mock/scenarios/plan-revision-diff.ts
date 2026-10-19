// A Job presenting plan v5 after a lot of v4 is built: two groups done, a third in progress with a task added and one renamed, and one task
// taken out. The thread's card draws the revision as its diff. Over the `one-session-view` scenario, with one of its Sessions given the plan.
// The walk `planRevisionDiff` plays it.

import type { Session, WorkGroup, WorkTask, WorkTaskState } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { planCall, withoutPlanCall } from "../plan-call";
import { s261OneSessionView as base } from "./one-session-view";

const QUIET = "one-view-2";

const task = (id: string, text: string, state: WorkTaskState): WorkTask => ({ id, text, state, seen: state !== "open" });

const GROUPS: readonly WorkGroup[] = [
  {
    id: "w1",
    agent: "rust-engineer",
    title: "Warm a bay before it is leased",
    paths: ["crates/fleet/src/pool.rs"],
    tasks: [task("W1", "Find the next free bay", "done"), task("W2", "Warm it in the background", "done"), task("W3", "Stop warming when the pool is full", "done")],
  },
  {
    id: "w2",
    agent: "protocol-engineer",
    title: "Warm state on the wire",
    paths: ["crates/ipc/src/bay.rs", "packages/protocol/src/bays.ts"],
    tasks: [task("W4", "Add warming to the bay reply", "done"), task("W5", "Mirror it in the TypeScript wire", "done")],
  },
  {
    id: "w3",
    agent: "bridge-engineer",
    title: "Show warming on the bay tile",
    paths: ["apps/desktop/src/renderer/src/cockpit/deck"],
    tasks: [
      task("W6", "Draw a warming bay", "done"),
      task("W7", "Keep the tile still while it warms", "working"),
      task("W8", "Say how long the warm took", "open"),
      task("W9", "Show warming in the bay pane", "open"),
    ],
  },
];

const revised = (one: Session): Session => ({
  ...one,
  title: "Warm the next bay",
  rows: [
    { id: "p-1", at: "13:20:00", kind: "message", from: { kind: "you" }, text: "Warm the next bay before it is leased, so a lease never waits on a cold one." },
    { id: "p-2", at: "14:12:30", kind: "message", from: { kind: "agent" }, text: "Plan v5 is recorded. Approve it, or tell me what to change." },
  ],
  waitingFor: [...withoutPlanCall(one.waitingFor), planCall(5, new Date().toISOString())],
  work: {
    workflow: { name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"], at: 1 },
    plan: {
      version: 5,
      scope: ["crates/fleet/src/pool.rs", "crates/ipc/src/bay.rs", "packages/protocol/src/bays.ts", "apps/desktop/src/renderer/src/cockpit/deck"],
      approved: false,
      after: "p-2",
      groups: GROUPS,
      changes: {
        added: ["W9"],
        renamed: [{ id: "W8", from: "Show how long warming took" }],
        removed: [{ group: "w3", text: "Add a warm button to the bay tile" }],
      },
    },
  },
});

export const s274PlanRevisionDiff: Scenario = {
  ...base,
  name: "plan-revision-diff",
  says: "A Job presenting plan v5 over a lot of finished v4 work: the unchanged groups close to a line and the changed group shows its marks",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(QUIET, revised);
      return store;
    },
  },
};
