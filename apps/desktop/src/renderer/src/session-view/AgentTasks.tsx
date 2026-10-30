// The agent's own task list in the order it made them, each its state mark (the plan task's own, with its tooltip) and its subject. The one being done
// breathes while the turn runs and is still once it has ended. The list is one stop on Tab, and the arrows, Home and End move through it.

import { useState, type KeyboardEvent } from "react";
import { TASK_GLYPH, TaskMark, Tooltip, taskMarkSaid, type TaskMarkState } from "@armada/components";
import type { SessionTodo } from "@armada/screens/src/draft/sessions";

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

const MARK_OF: Record<SessionTodo["state"], TaskMarkState> = { done: "done", doing: "working", todo: "open" };

function Mark({ state, breathing }: { state: TaskMarkState; breathing: boolean }) {
  if (state !== "working" || !breathing) return <TaskMark state={state} />;
  const said = taskMarkSaid(state);
  return (
    <Tooltip asChild label={said}>
      <span className="armada-session-mark armada-task-mark" data-state={state} data-pulsing role="img" aria-label={said}>
        <TASK_GLYPH.working size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export function AgentTasks({ todos, working }: { todos: readonly SessionTodo[]; working: boolean }) {
  // The row with the focus is the list's tab stop, so Tab leaves from where the arrows left it.
  const [stop, setStop] = useState(0);
  return (
    <ul className="sv-tasks sv-agent-tasks" aria-label="Tasks" data-own-arrows="" onKeyDown={move}>
      {todos.map((todo, i) => {
        const state = MARK_OF[todo.state];
        return (
          <li key={`${i}-${todo.text}`} className="sv-task sv-agent-tasks__row" data-state={state} tabIndex={i === Math.min(stop, todos.length - 1) ? 0 : -1} onFocus={() => setStop(i)}>
            <Mark state={state} breathing={working} />
            <span className="sv-agent-tasks__subject">{todo.text}</span>
          </li>
        );
      })}
    </ul>
  );
}
