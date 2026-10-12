// What a whole-window note records of the app's state, read from what Bridge holds and the page.

import { openJobIn } from "@armada/jobs";

import type { AppContext } from "../../../shared/annotations";
import type { BridgeState } from "../../../shared/bridge";
import { gridOf, summaryOf } from "../doctor/grid";
import { openSessionIn } from "../open-session";

/** The slices of Bridge's state the context reads. */
export type ContextState = Pick<BridgeState, "connection" | "health" | "jobs" | "sessions">;

/** Doctor's read, the open Job and the open Session, as Fleet last said them. */
export function contextOf(state: ContextState, doc: Document = document): AppContext {
  // Doctor's own spelling of its grid. The connection's state word stands in for the headline the app draws.
  const grid = gridOf(state.health, state.connection, state.connection.state);
  const context: AppContext = {
    connection: state.connection.state,
    doctor: { summary: summaryOf(grid), rows: grid.rows.map(({ module, result, detail }) => ({ module, result, detail })) },
  };
  const jobId = openJobIn(doc);
  const job = jobId === undefined ? undefined : state.jobs.find((one) => one.id === jobId);
  if (job !== undefined) {
    context.job = {
      id: job.id,
      handle: job.handle,
      title: job.title,
      status: job.status,
      ...(job.reason?.named === undefined ? {} : { reason: job.reason.named }),
      ...(job.current_step_id === undefined ? {} : { step: job.current_step_id }),
      ...(job.assigned_drone === undefined ? {} : { drone: job.assigned_drone }),
      workflow: job.workflow_id,
    };
  }
  const sessionId = openSessionIn(doc);
  const session = sessionId === undefined || state.sessions.state !== "read" ? undefined : state.sessions.sessions.find((one) => one.id === sessionId);
  if (session !== undefined) {
    context.session = {
      id: session.id,
      ...(session.title === undefined ? {} : { title: session.title }),
      state: session.state,
      harness: session.harness,
      cwd: session.cwd,
      ...(session.end_reason === undefined ? {} : { endReason: session.end_reason }),
    };
  }
  return context;
}
