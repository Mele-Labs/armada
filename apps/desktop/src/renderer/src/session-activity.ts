// What a Session is doing, read off `Session.activity` for the three places that draw it: a list row's
// line, the band over the box, and a Cockpit tile. Todos are the agent's report and `now` is what Fleet
// detected, so the todos only ever add to `now`. Nothing is drawn for a Session whose turn is not
// running: an idle agent is doing nothing, whatever its list last said.

import type { WorkingTodo } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

const running = (session: Session): boolean => session.dead === undefined && session.turn.state === "working";

/** The todos in the order the band draws them: the one being done, then those still to do, then those done. */
export function todosOf(session: Session): WorkingTodo[] {
  if (!running(session)) return [];
  const todos = (session.activity?.todos ?? []).map((one, at): WorkingTodo => ({ key: `${at}`, ...one }));
  return (["doing", "todo", "done"] as const).flatMap((state) => todos.filter((one) => one.state === state));
}

/** Its detected action, for a list row. */
export function nowOf(session: Session): string | undefined {
  return running(session) ? session.activity?.now : undefined;
}

/** One line for a tile: the todo being done, else the detected action. */
export function lineOf(session: Session): string | undefined {
  return todosOf(session).find((one) => one.state === "doing")?.text ?? nowOf(session);
}

/** The Now line: the task being worked, the detected action, and the agent's last sentence. Nothing unless the turn runs. */
export type NowLine = { task?: string; action?: string; said?: string };

/** The task the agent is on: the plan's working task, else the one on its own list. */
function workingTaskOf(session: Session): string | undefined {
  const planned = session.work?.plan?.groups.flatMap((group) => group.tasks).find((task) => task.state === "working");
  return planned?.text ?? session.work?.tasks?.find((task) => task.state === "working")?.subject;
}

/** What the Session is on, for the header and its list row: the task, then the action Fleet detected, then its last words. Absent while nothing runs or nothing is known. */
export function nowLineOf(session: Session): NowLine | undefined {
  if (!running(session)) return undefined;
  const task = workingTaskOf(session);
  const action = session.activity?.now;
  const said = session.activity?.said;
  if (task === undefined && action === undefined && said === undefined) return undefined;
  return { ...(task === undefined ? {} : { task }), ...(action === undefined ? {} : { action }), ...(said === undefined ? {} : { said }) };
}

/** The first line of `nowLineOf` as one string: the task, and the action after a separator. */
export function nowSaidOf(session: Session): string | undefined {
  const now = nowLineOf(session);
  if (now === undefined) return undefined;
  return [now.task, now.action].filter((one) => one !== undefined).join(" · ") || undefined;
}

/** One line per subagent running under the Session: what it was sent to do and what it is on now. Empty while nothing runs. */
export function subagentLinesOf(session: Session): readonly { id: string; label: string; now?: string }[] {
  return running(session) ? (session.activity?.subagents ?? []) : [];
}
