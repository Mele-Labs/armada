// The approved Blueprint's Now view: the task finished last, every task being worked with what the turn is doing under the first of them, and the
// task waiting next. A task that failed stands with the ones being worked, since it is where the build is stuck. A task being worked ends in the chip of the issue its
// group works. The whole plan is a press away (`BlueprintPanel.tsx`).

import type { Session } from "@armada/screens/src/draft/sessions";

import { issueOfGroup } from "./IssuesBlock";
import { GroupIssue, hereOf, TaskLine } from "./PlanGroup";
import type { ViewGroup } from "./plan-marks";

const finished = (state: string) => state === "done" || state === "handed_in";

export function NowList({ groups, workingGroup, now, session, manifestId }: { groups: readonly ViewGroup[]; workingGroup: string | undefined; now: string | undefined; session: Session; manifestId: string | undefined }) {
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
      {rows.map((task) => {
        const named = task.state === "working" ? issueOfGroup(session, groups.find((group) => group.tasks.some((one) => one.id === task.id))?.issue) : undefined;
        return <TaskLine key={task.id} task={task} here={task.state === "working"} now={task.id === liveTask ? now : undefined} {...(named === undefined ? {} : { issue: <GroupIssue issue={named.issue} manifestId={manifestId} /> })} />;
      })}
    </ul>
  );
}
