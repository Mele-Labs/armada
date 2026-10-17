// Progress with no plan: the subagents at work, each its working mark (the plan's own, with its tooltip) and its title.

import { TaskMark } from "@armada/components";

import type { SubagentAtWork } from "./subagents-at-work";
import "./subagents-at-work.css";

export function SubagentsAtWork({ rows }: { rows: readonly SubagentAtWork[] }) {
  return (
    <ul className="sv-subagents" aria-label="Subagents">
      {rows.map((one) => (
        <li key={one.id} className="sv-subagents__row">
          <TaskMark state="working" />
          <span className="sv-subagents__task">{one.task}</span>
          {one.agentType === undefined ? null : <code className="sv-subagents__type">{one.agentType}</code>}
        </li>
      ))}
    </ul>
  );
}
