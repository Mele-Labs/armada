// The agent's own task list on the Job's spine, drawn once. Three Jobs over `one-session-view`: a Feature workflow at Walk with an approved plan, whose
// list repeats three plan tasks (matched, so drawn by the plan alone) and adds two the plan lacks; a Feature workflow at Propose with no plan and a list
// under way; and a custom Job with no workflow whose turn has ended, so the list stays with its working mark still. The walk `spine-agent-tasks` plays it.

import type { Session, SessionTodo, WorkflowChoice } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s261OneSessionView as base } from "./one-session-view";

const MINE = "one-view-1";
const QUIET = "one-view-2";
const BUSY = "one-view-3";

const FEATURE: WorkflowChoice = { id: "Feature", name: "Feature", steps: ["Propose", "Build", "Walk", "Deliver"] };

// The plan's tasks are A1, A2 and A3 (`one-session-view`); a todo that carries one is the plan row's own and is not drawn again.
const AT_WALK: readonly SessionTodo[] = [
  { text: "Read the retention setting", state: "done" },
  { text: "Rotate the log file at midnight", state: "done", task: "A1" },
  { text: "Keep seven files", state: "done", task: "A2" },
  { text: "List rotated files in System", state: "doing", task: "A3" },
  { text: "Document the retention", state: "todo" },
];

const AT_PROPOSE: readonly SessionTodo[] = [
  { text: "Read the migration runner", state: "done" },
  { text: "List the migration files", state: "done" },
  { text: "Rename the files to their order", state: "doing" },
  { text: "Update the loader tests", state: "todo" },
];

const CUSTOM: readonly SessionTodo[] = [
  { text: "Read the lease code", state: "done" },
  { text: "Retry when two Jobs ask for one bay", state: "doing" },
  { text: "Cover the retry in the pool tests", state: "todo" },
  { text: "Update the pool contract", state: "todo" },
];

const walking = (one: Session): Session => ({
  ...one,
  work: { ...one.work, proposal: { workflow: FEATURE.name }, workflow: { ...FEATURE, at: 2 } },
  activity: { todos: AT_WALK },
});

const proposing = (one: Session): Session => ({
  ...one,
  turn: { state: "working" },
  standing: "working",
  work: { proposal: { workflow: FEATURE.name }, workflow: { ...FEATURE, at: 0 } },
  activity: { todos: AT_PROPOSE },
});

const custom = (one: Session): Session => ({
  ...one,
  turn: { state: "idle" },
  standing: "idle",
  work: { proposal: { workflow: null } },
  activity: { todos: CUSTOM },
});

export const s340SpineAgentTasks: Scenario = {
  ...base,
  name: "spine-agent-tasks",
  says: "The agent's task list on the spine: under the Walk step after the plan's tasks, under the Propose step with no plan, and under the Agent row of a custom Job whose turn has ended",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(BUSY, walking);
      store.edit(QUIET, proposing);
      store.edit(MINE, custom);
      return store;
    },
  },
};
