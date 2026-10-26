// The approved Blueprint's Now view: the task finished last, every task being worked with what the turn is doing under the first of them, and the
// task waiting next. A task that failed stands with the ones being worked, since it is where the build is stuck. The whole plan is a press away (`BlueprintPanel.tsx`).

import { hereOf, TaskLine } from "./PlanGroup";
import type { ViewGroup } from "./plan-marks";

const finished = (state: string) => state === "done" || state === "handed_in";

export function NowList({ groups, workingGroup, now }: { groups: readonly ViewGroup[]; workingGroup: string | undefined; now: string | undefined }) {
  const tasks = groups.flatMap((group) => group.tasks.filter((task) => task.removed !== true));
  const last = [...tasks].reverse().find((task) => finished(task.state));
  const going = tasks.filter((task) => task.state === "working" || task.state === "failed");
  const next = tasks.find((task) => task.state === "open");
  const lit = groups.find((group) => group.id === workingGroup);
  const liveTask = lit === undefined ? undefined : hereOf(lit);
  const rows = [...(last === undefined ? [] : [last]), ...going, ...(next === undefined ? [] : [next])];
  if (rows.length === 0) return null;
  return (
    <ul className="sv-group__tasks sv-now" aria-label="Now">
      {rows.map((task) => (
        <TaskLine key={task.id} task={task} here={task.state === "working"} now={task.id === liveTask ? now : undefined} />
      ))}
    </ul>
  );
}
