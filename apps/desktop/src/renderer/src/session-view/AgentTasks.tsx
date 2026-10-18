// Progress with no plan: the agent's own task list in the order it made them, each its state mark (the plan task's own, with its tooltip) and its
// subject. The list is one stop on Tab, and the arrows, Home and End move through it.

import { useState, type KeyboardEvent } from "react";
import { TaskMark } from "@armada/components";
import type { AgentTask } from "@armada/screens/src/draft/sessions";

import "./agent-tasks.css";

function move(event: KeyboardEvent<HTMLUListElement>) {
  const keys: Record<string, (at: number, last: number) => number> = { ArrowDown: (at, last) => Math.min(at + 1, last), ArrowUp: (at) => Math.max(at - 1, 0), Home: () => 0, End: (_, last) => last };
  const to = keys[event.key];
  if (to === undefined) return;
  const rows = [...event.currentTarget.querySelectorAll<HTMLElement>(":scope > li")];
  const at = rows.findIndex((one) => one === document.activeElement);
  rows[to(at, rows.length - 1)]?.focus();
  event.preventDefault();
}

export function AgentTasks({ tasks }: { tasks: readonly AgentTask[] }) {
  // The row with the focus is the list's tab stop, so Tab leaves from where the arrows left it.
  const [stop, setStop] = useState(0);
  return (
    <ul className="sv-tasks sv-agent-tasks" aria-label="Tasks" data-own-arrows="" onKeyDown={move}>
      {tasks.map((task, i) => (
        <li key={task.id} className="sv-task sv-agent-tasks__row" data-state={task.state} tabIndex={i === Math.min(stop, tasks.length - 1) ? 0 : -1} onFocus={() => setStop(i)}>
          <TaskMark state={task.state} />
          <span className="sv-agent-tasks__subject">{task.subject}</span>
        </li>
      ))}
    </ul>
  );
}
