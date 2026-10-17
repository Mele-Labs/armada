// What a whole-window note records of the app's state, read from what Bridge holds and the page.

import { sessionFacts, type AppContext } from "../../../shared/annotations";
import type { BridgeState } from "../../../shared/bridge";
import { gridOf, summaryOf } from "../doctor/grid";
import { openSessionIn } from "../open-session";

/** The slices of Bridge's state the context reads. */
export type ContextState = Pick<BridgeState, "connection" | "health" | "sessions">;

/** Doctor's read and the open Session, as Fleet last said them. */
export function contextOf(state: ContextState, doc: Document = document): AppContext {
  // Doctor's own spelling of its grid. The connection's state word stands in for the headline the app draws.
  const grid = gridOf(state.health, state.connection, state.connection.state);
  const context: AppContext = {
    connection: state.connection.state,
    doctor: { summary: summaryOf(grid), rows: grid.rows.map(({ module, result, detail }) => ({ module, result, detail })) },
  };
  const sessionId = openSessionIn(doc);
  const session = sessionId === undefined || state.sessions.state !== "read" ? undefined : state.sessions.sessions.find((one) => one.id === sessionId);
  if (session !== undefined) context.session = sessionFacts(session);
  return context;
}
