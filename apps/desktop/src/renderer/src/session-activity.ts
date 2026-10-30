// What a Session is doing, read off `Session.activity` for the two places that draw it from a running turn: a list row's
// line and a Cockpit tile. The Job's spine draws the whole list from `activity.todos` itself. Todos are the agent's report and `now` is what Fleet
// detected, so the todos only ever add to `now`. Nothing is drawn for a Session whose turn is not
// running: an idle agent is doing nothing, whatever its list last said.

import type { TurnAccount, WorkingTodo } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

const running = (session: Session): boolean => session.dead === undefined && session.turn.state === "working";

/** The todos of a running turn: the one being done first, then those still to do, then those done. */
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

/** The group of the approved plan the account's now line belongs to, while the turn runs. Nothing without an approved plan, a now line or a match. */
export function accountGroupOf(session: Session): { id: string; title: string } | undefined {
  const account = session.account;
  const plan = session.work?.plan;
  if (account?.group === undefined || account.now === undefined || plan === undefined || !plan.approved || !running(session)) return undefined;
  return plan.groups.find((one) => one.id === account.group);
}

/**
 * The turn's running account as Progress draws it: while the turn runs, and once it has ended, all done, until his next message. Ended,
 * it has no line for now or next.
 */
export function accountOf(session: Session): TurnAccount | undefined {
  const account = session.account;
  if (account === undefined) return undefined;
  if (running(session)) {
    const group = accountGroupOf(session);
    return { done: account.done, ...(account.now === undefined ? {} : { now: account.now }), ...(group === undefined ? {} : { of: group.title }), ...(account.next === undefined ? {} : { next: account.next }) };
  }
  const at = session.rows.findIndex((row) => row.id === account.for);
  const answered = at >= 0 && session.rows.slice(at + 1).some((row) => row.kind === "message" && row.from.kind === "you");
  return answered ? undefined : { done: account.done };
}
