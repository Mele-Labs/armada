// A repository's open issues, as main asks Fleet for them: one request, and nothing kept here, since
// Fleet holds the list. #91. And the Autopilot that works them (#2092), which Fleet holds too.

import type { Autopilot, AutopilotSettings, IssueList, Outcome } from "@armada/protocol";
import type { AutopilotActed } from "../shared/api/autopilot";
import type { IssuesRead } from "../shared/api/issues";
import { ask, MODEL_CALL_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class IssuesHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async list(manifestId: string, refresh: boolean): Promise<IssuesRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const path = `/issues/${encodeURIComponent(manifestId)}${refresh ? "?refresh=true" : ""}`;
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as IssueList };
  }

  /** One Autopilot act. A sentence read and a set-aside pick dispatched are model calls, so they wait longer. */
  async autopilot<T = Autopilot>(method: "GET" | "POST", path: string, body?: unknown, slow = false): Promise<AutopilotActed<T>> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = slow ? await ask(port, method, path, body, MODEL_CALL_MS) : await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as T };
  }

  /** The path of one repository's Autopilot, and of something under it. */
  static autopilotOf(manifestId: string, rest = ""): string {
    return `/autopilots/${encodeURIComponent(manifestId)}${rest}`;
  }

  async proposeAutopilot(manifestId: string, sentence: string, localNow: string): Promise<AutopilotActed<AutopilotSettings>> {
    const body = { sentence, local_now: localNow };
    return await this.autopilot<AutopilotSettings>("POST", IssuesHost.autopilotOf(manifestId, "/proposal"), body, true);
  }
}
