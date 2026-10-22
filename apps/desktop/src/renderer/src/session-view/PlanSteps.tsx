// The plan in Progress as a stepper rather than equal cards: what is done is one muted line, the group being worked is open and raised with its
// tasks, and what is to come is one muted line each. `stepsOf` says which is which; `StepSegments` is the plan's position as a row of marks, with no count.

import { Tooltip } from "@armada/components";

import type { Step } from "./PlanGroup";
import type { ViewGroup } from "./plan-marks";

const finished = (group: ViewGroup): boolean => group.tasks.length > 0 && group.tasks.every((task) => task.state === "done" || task.state === "handed_in");

/**
 * Each group's place in the plan. Until the plan is approved nothing is being worked, so every group is to come. The group the account is on is
 * current, and so is one with a task running or failed when the account names none; a group whose tasks are all done is done.
 */
export function stepsOf(groups: readonly ViewGroup[], approved: boolean, working: string | undefined): Map<string, Step> {
  const steps = new Map<string, Step>();
  for (const group of groups) {
    const live = group.tasks.some((task) => task.state === "working" || task.state === "failed");
    steps.set(group.id, !approved ? "upcoming" : group.id === working ? "current" : finished(group) ? "done" : live ? "current" : "upcoming");
  }
  return steps;
}

const SAID: Record<Step, string> = { done: "Done", current: "Working on this now", upcoming: "Not started" };

/** The plan's groups as a row of small marks, each in its step's hue and named by its tooltip. */
export function StepSegments({ groups, steps }: { groups: readonly ViewGroup[]; steps: ReadonlyMap<string, Step> }) {
  return (
    <span className="sv-segs">
      {groups.map((group) => {
        const step = steps.get(group.id) ?? "upcoming";
        const said = `${group.title} · ${SAID[step]}`;
        return (
          <Tooltip key={group.id} label={said}>
            <span className="sv-segs__seg" data-step={step} role="img" aria-label={said} />
          </Tooltip>
        );
      })}
    </span>
  );
}
