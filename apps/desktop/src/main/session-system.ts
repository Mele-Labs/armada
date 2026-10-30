// One Job's system, as Fleet answers `GET /sessions/system`, and one process of it stopped with
// `POST /sessions/stop_process`. Nothing is kept here; the renderer decides when to ask again.

import type { Outcome, ProcessStopped, SessionSystem } from "@armada/protocol";
import type { ProcessStopRead, ProcessTarget, SessionSystemRead } from "../shared/api/session-system";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** The query string for one Job, encoded. */
export function sessionSystemPath(sessionId: string): string {
  return `/sessions/system?session_id=${encodeURIComponent(sessionId)}`;
}

export class SessionSystemHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(sessionId: string): Promise<SessionSystemRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", sessionSystemPath(sessionId));
    return answer.ok === true ? { ok: true, value: answer.body as SessionSystem } : { ok: false, outcome: answer.outcome };
  }

  async stop(sessionId: string, target: ProcessTarget): Promise<ProcessStopRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/sessions/stop_process", { session_id: sessionId, ...target });
    return answer.ok === true ? { ok: true, value: answer.body as ProcessStopped } : { ok: false, outcome: answer.outcome };
  }
}
