// What a task's own Drone is doing, for Plan's task sheet. `#1536`.
//
// **Read only where the task has a Drone of its own.** That is slices 1 and 5
// of `docs/spikes/022`: until Fleet puts a Drone on each task, a task's turns,
// cost and transcript are the Job's one Drone's, and a sentence about "its
// agent" would be about a Drone that is not this task's. So today these read
// off the mock's draft and nothing else, and on real Fleet the sheet draws none
// of them.
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
 * What a task has spent. **Turns while it runs, and the cost only once its own
 * agent stopped** — a live figure would be invented, since cost reaches Armada
 * on a session's last line.
 */
function spentSaid(task: TaskView): string | undefined {
  const parts: string[] = [];
  if (task.turns !== undefined) parts.push(`${task.turns} turns`);
  if (task.cost_micros !== undefined) parts.push(money(task.cost_micros));
  return parts.length === 0 ? undefined : parts.join(" · ");
}

/**
 * What the task's agent is doing now, as a sentence. **Working and done only**:
 * a failed task's reason and a dropped one's are drawn by the sheet already,
 * and an open task has no agent to say anything about.
 */
export function doingOfTask(task: TaskView): string | undefined {
  if (!hasOwnDrone(task)) return undefined;
  const spent = spentSaid(task);
  switch (task.state) {
    case "working":
      return spent === undefined
        ? "Its agent is working. Nothing it has spent can be read until that agent stops."
        : `Its agent is working — ${spent} so far. What it cost reads once that agent stops.`;
    case "done":
      return spent === undefined ? "Its agent has stopped and the work is in." : `Its agent stopped after ${spent}.`;
    default:
      return undefined;
  }
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
