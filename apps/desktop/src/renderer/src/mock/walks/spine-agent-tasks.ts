// The agent's task list on the Job's spine, drawn once. Under the Walk step it follows the plan's tasks, and only the items the plan lacks are added; with no plan it
// sits under the Propose step; a custom Job keeps it under the Agent row, after the turn has ended too. Over `spine-agent-tasks`.

import { deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const walking = region("Walk", { exact: true });
const proposing = region("Propose", { exact: true });
const now = region("Now", { exact: true });

const spineAgentTasks = walk("spine-agent-tasks", [
  { press: deckRow("Rotate the audit log"), say: "A Feature Job at its Walk step, with an approved plan" },
  { look: inside(walking, text("Document the retention")), say: "After the plan's tasks, the items the plan lacks. The ones it has are not drawn twice" },
  toSessions,
  { press: deckRow("Name the store migrations"), say: "A Feature Job at its first step, with no plan" },
  { look: inside(proposing, role("list", "Tasks")), say: "The whole list sits under the current step" },
  toSessions,
  { press: deckRow("Retry the slot lease"), say: "A custom Job, its turn ended" },
  { hover: inside(inside(now, role("list", "Tasks")), role("img", "Working")), say: "The list stays, and the working mark is still" },
]);

export { spineAgentTasks as "spine-agent-tasks" };
