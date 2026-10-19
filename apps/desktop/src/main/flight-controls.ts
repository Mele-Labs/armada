// The flight controls Kit holds, and a shell line run for a Job, as Fleet answers `/kit/flight_controls` and `/sessions/shell`.
// Nothing is kept here: Fleet reads and writes `~/.armada/flight-controls.toml`, and the renderer decides when to ask again.

import type { FlightControls, FlightControlsRead, Outcome, RunSessionShell, SessionShellRan } from "@armada/protocol";
import type { FlightControlsAnswer, SessionShellAnswer } from "../shared/api/flight-controls";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/**
 * How long a shell line is waited for. **Fleet stops one at 120 seconds** and answers with `timed_out`; this is that plus five, so what
 * arrives is Fleet's word and not Bridge's abort. Nothing generates it from the Rust constant, so the two are coupled by this comment.
 */
export const SHELL_MS = 125_000;

export class FlightControlsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(): Promise<FlightControlsAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", "/kit/flight_controls");
    return answer.ok === true ? { ok: true, value: answer.body as FlightControlsRead } : { ok: false, outcome: answer.outcome };
  }

  async save(controls: FlightControls): Promise<FlightControlsAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/kit/flight_controls", controls);
    return answer.ok === true ? { ok: true, value: answer.body as FlightControlsRead } : { ok: false, outcome: answer.outcome };
  }

  async runShell(sessionId: string, command: string): Promise<SessionShellAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const body: RunSessionShell = { session_id: sessionId, command };
    const answer = await ask(port, "POST", "/sessions/shell", body, SHELL_MS);
    return answer.ok === true ? { ok: true, value: answer.body as SessionShellRan } : { ok: false, outcome: answer.outcome };
  }
}
