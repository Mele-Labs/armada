// What Fleet sees the Job doing, worked out from its tool calls and not from the agent's task marks, which lag the work. The line is the one the thread
// reads a run of calls as (`callsLine`): the calls since the last thing the agent said or the person sent.

import { callsLine } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

/** The line for the run of tool calls that ends the thread, while the turn runs. Nothing when the thread does not end in calls. */
export function detectedOf(session: Session): string | undefined {
  if (session.dead !== undefined || session.turn.state !== "working") return undefined;
  const run: { text: string }[] = [];
  for (let at = session.rows.length - 1; at >= 0; at--) {
    const row = session.rows[at]!;
    if (row.kind === "tool") run.unshift(row);
    else if (run.length > 0 || row.kind === "message") break;
  }
  const line = run.length === 0 ? "" : callsLine(run, true);
  return line === "" ? undefined : line;
}
