// An open Session with no plan, five tasks on the agent's own list (two done, one working, two open) and one subagent at work. Over the
// `one-session-view` scenario, with one of its Sessions given them. The walk `tasksWithoutPlan` plays it.

import type { Session, SessionTodo } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const QUIET = "one-view-2";

const TODOS: readonly SessionTodo[] = [
  { text: "Read the migration runner", state: "done" },
  { text: "List the migration files", state: "done" },
  { text: "Rename the files to their order", state: "doing" },
  { text: "Update the loader tests", state: "todo" },
  { text: "Run the store checks", state: "todo" },
];

const working = (one: Session): Session => ({
  ...one,
  turn: { state: "working" },
  standing: "working",
  attachments: [...one.attachments, { kind: "subagent", id: "sa-loader", task: "Find names the loader hardcodes", state: "running", agentType: "bridge-engineer" }],
  work: { proposal: { workflow: null } },
  activity: { todos: TODOS },
});

export const s281TasksWithoutPlan: Scenario = {
  ...base,
  name: "tasks-without-plan",
  says: "A Session with no plan: the spine lists the agent's tasks in the order made, and its subagent at work under them",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(QUIET, working);
      return store;
    },
  },
};
