// What a task's own Drone is doing, for Plan's task sheet. `#1536`.
//
// **Read only where the task has a Drone of its own**: a step declaring
// `drone_per_task` (protocol 23.1). Elsewhere a task's turns, cost and
// transcript are the step's one Drone's, and a sentence about "its agent"
// would be about a Drone that is not this task's.
//
// **Cost only once that agent stopped** (`#1530`, 22 Sep). Turns are live and a
// figure for spend is not, so a running task shows turns and nothing else.
//
// Restored from `implement-task.ts`, which #1757 deleted with the Workflow
// panel that drew it.

import type { Turn } from "@armada/protocol";

import type { TaskView } from "./draft/task";
import { money } from "./facts";
import { editsIn } from "./task-files";

/** Whether the task's own facts are its own Drone's. */
export function hasOwnDrone(task: TaskView): boolean {
  return task.treatment === "own_drone" && task.drone_id !== undefined;
}

/**
 * What the task's own agent has spent, as bare facts: `14 turns` while it
 * works, `27 turns · ~$1.90` once it stopped — handed in or done. **Not failed
 * or dropped**: the sheet already says why either stopped, and an open task has
 * no agent. Absent where there is nothing to say.
 *
 * **No cost while it runs**: cost reaches Armada on a session's last line, so a
 * live figure would be invented.
 */
export function doingOfTask(task: TaskView): string | undefined {
  const stopped = task.state === "handed_in" || task.state === "done";
  if (!hasOwnDrone(task) || (task.state !== "working" && !stopped)) return undefined;
  const parts: string[] = [];
  if (task.turns !== undefined) parts.push(`${task.turns} turns`);
  if (stopped && task.cost_micros !== undefined) parts.push(money(task.cost_micros));
  return parts.length === 0 ? undefined : parts.join(" · ");
}

/**
 * The last file the task's Drone wrote, with the size of that edit. **From
 * that Drone's own transcript**, so every edit in it is this task's. Absent
 * where no transcript is served — today's Fleet serves none.
 */
export function lastEditOf(transcript: readonly Turn[] | undefined): { path: string; says?: string } | undefined {
  if (transcript === undefined) return undefined;
  const last = editsIn(transcript, []).at(-1);
  if (last === undefined) return undefined;
  const sizes = [
    last.added === undefined || last.added === 0 ? undefined : `+${last.added}`,
    last.deleted === undefined || last.deleted === 0 ? undefined : `−${last.deleted}`,
  ].filter((one): one is string => one !== undefined);
  return { path: last.path, ...(sizes.length === 0 ? {} : { says: sizes.join(" ") }) };
}
