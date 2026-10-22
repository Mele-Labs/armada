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

/** The account's line for what the turn is on now, in his words, for a list row. Nothing unless the turn runs. */
export function accountNowOf(session: Session): string | undefined {
  return running(session) ? session.account?.now : undefined;
}
