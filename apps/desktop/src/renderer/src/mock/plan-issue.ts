// The owner's two variants of how a plan group shows the issue it names (`?plan=chip|absorb`); with none, chip. Chip marks the group with
// its issue and leaves the issues block as it is. Absorb makes the group the issue's place: its pull requests stand in the group and the
// block lists only what no group names. Mock only: the one he chooses becomes the plan's drawing and this file goes.

import { absorbPlanIssues } from "../session-view/IssuesBlock";

export function takeThePlanIssue(variant: string | null): void {
  if (variant === "absorb") absorbPlanIssues();
}
