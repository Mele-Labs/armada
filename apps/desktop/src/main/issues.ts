// A repository's open issues, as main asks Fleet for them: one request, and nothing kept here, since
// Fleet holds the list. #91.

import type { IssueList, Outcome } from "@armada/protocol";
import type { IssuesRead } from "../shared/api/issues";
import { ask } from "./request";

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
}
