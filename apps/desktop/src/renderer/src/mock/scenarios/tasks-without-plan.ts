// An open Session with no plan, five tasks on the agent's own list (two done, one working, two open) and one subagent at work. Over the
// `one-session-view` scenario, with one of its Sessions given them. The walk `tasksWithoutPlan` plays it.

import type { AgentTask, Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const QUIET = "one-view-2";

const TASKS: readonly AgentTask[] = [
  { id: "at-1", subject: "Read the migration runner", state: "done" },
  { id: "at-2", subject: "List the migration files", state: "done" },
  { id: "at-3", subject: "Rename the files to their order", state: "working" },
  { id: "at-4", subject: "Update the loader tests", state: "open" },
  { id: "at-5", subject: "Run the store checks", state: "open" },
];

const working = (one: Session): Session => ({
  ...one,
  turn: { state: "working" },
  standing: "working",
  attachments: [...one.attachments, { kind: "subagent", id: "sa-loader", task: "Find names the loader hardcodes", state: "running", agentType: "bridge-engineer" }],
  work: { proposal: { workflow: null }, tasks: TASKS },
});

export const s281TasksWithoutPlan: Scenario = {
  ...base,
  name: "tasks-without-plan",
  says: "A Session with no plan: the Blueprint lists the agent's tasks in the order made, and its subagent at work under them",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(QUIET, working);
      return store;
    },
  },
};
