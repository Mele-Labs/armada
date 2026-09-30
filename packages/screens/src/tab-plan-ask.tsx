// Asking the plan's Drone to rethink part of the plan, in a person's own
// words. `#1552`.
//
// **Its own file, for `Redirect.tsx`'s reason.** Which acts a destination
// offers is one subject, and what a single act says to the Drone is another —
// this is the second, and `plan-review.tsx` is the first.
//
// **The ask reaches the Drone as a redirect**, which is the one command that
// turns into a session already open. Nothing new goes on the wire: a proposed
// change is a person saying something to the Drone that wrote the plan, and
// `redirect` is what that has always been. **Every other change is a person's
// own**, made directly through Fleet —
// `.claude/decisions/2026-09-30-plan-edits-go-straight-through-fleet.md`.

import type { GroupView } from "./draft/group";

/** Which group the ask is on, or `undefined` where the plan has no such group. */
function groupOf(groups: readonly GroupView[], id: string): GroupView | undefined {
  return groups.find((one) => one.id === id);
}

/** A proposed change to one task, in the terms the Drone may refuse. */
export function rewriteInstruction(taskId: string, note: string): string {
  return [
    `A change to the plan you recorded, on ${taskId}: ${note}`,
    "This is a request about the split, not about the code. Where the task is written the way it is for a reason the plan does not give, refuse it and say what that reason is.",
  ].join("\n\n");
}

/**
 * A proposed change to one group, on the same terms. **It names the tasks
 * the group holds**, so the Drone reads the change against what it wrote
 * rather than against an ordinal it may have renumbered since.
 */
export function proposeInstruction(groups: readonly GroupView[], id: string, note: string): string {
  const group = groupOf(groups, id);
  const ids = group?.tasks.map((task) => task.id).join(", ") ?? "";
  const on = group === undefined ? "on a group" : `on group ${group.ordinal}${ids === "" ? "" : ` (${ids})`}`;
  return [
    `A change to the plan you recorded, ${on}: ${note}`,
    "This is a request about the split, not about the code. Where the group is split the way it is for a reason the plan does not give, refuse it and say what that reason is.",
  ].join("\n\n");
}
