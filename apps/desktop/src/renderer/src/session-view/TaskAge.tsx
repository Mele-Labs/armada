// How long an in-progress mark has stood unchanged: a quiet reading at the right of the task, with a tooltip that names it. A mark untouched for `STALE_AFTER` is
// dimmed by its row (`story.css`, `data-stale`); the agent's marks lag the work, so an old one is a prompt to look and not a claim.

import { Readout, Tooltip, useNow } from "@armada/components";

import type { ViewGroup } from "./plan-marks";

/** A mark untouched this long dims. */
export const STALE_AFTER_MS = 30 * 60_000;

/** One unit, the largest that fits: `40 s`, `12 m`, `2 h`, `3 d`. */
export function idleText(ms: number): string {
  const seconds = Math.floor(Math.max(0, ms) / 1000);
  if (seconds < 60) return `${seconds} s`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} m`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours} h` : `${Math.floor(hours / 24)} d`;
}

/** The task's mark is one the agent set to in progress, and the age of its last change is known. */
export const agedAt = (task: ViewGroup["tasks"][number]): number | undefined => {
  if (task.state !== "working" || task.changedAt === undefined) return undefined;
  const at = Date.parse(task.changedAt);
  return Number.isNaN(at) ? undefined : at;
};

/** The group's marks in progress, the freshest change first; a group with none carries no age. */
export const freshestOf = (group: ViewGroup): number => Math.max(-Infinity, ...group.tasks.flatMap((task) => agedAt(task) ?? []));

export function TaskAge({ since }: { since: number }) {
  const now = useNow(5_000);
  return (
    <Tooltip asChild label="Since the agent last updated this task">
      <Readout fixed className="sv-group__task-age">{idleText(now - since)}</Readout>
    </Tooltip>
  );
}

/** Whether the mark has stood unchanged past the threshold, read on the same clock as its readout. */
export function useStale(since: number | undefined): boolean {
  const now = useNow(5_000);
  return since !== undefined && now - since > STALE_AFTER_MS;
}
