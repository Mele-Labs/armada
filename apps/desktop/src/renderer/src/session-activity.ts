// What a Session is doing, read off `Session.activity` for the three places that draw it: a list row's
// line, the band over the box, and a Cockpit tile. Todos are the agent's report and `now` is what Fleet
// detected, so the todos only ever add to `now`. Nothing is drawn for a Session whose turn is not
// running: an idle agent is doing nothing, whatever its list last said.

import type { WorkingTodo } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

/** How long a running turn may go with no tool call before it is marked. The mock's own figure. */
export const QUIET_AFTER_MS = 10 * 60_000;

const running = (session: Session): boolean => session.dead === undefined && session.turn.state === "working";

/** The tooltip on a running turn that has gone quiet; nothing for one that has not. */
export function quietOf(session: Session, now = Date.now()): string | undefined {
  const since = session.activity?.quietSince;
  if (!running(session) || since === undefined || now - Date.parse(since) < QUIET_AFTER_MS) return undefined;
  const at = new Date(since).toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" });
  return `No tool call since ${at}`;
}

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
