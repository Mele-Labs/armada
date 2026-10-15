// One bay's history, as Fleet answers `GET /slot_history`: one request for a repository and a slot.
// Nothing is kept here; Fleet keeps the history and the renderer decides when to ask again.

import type { BayHistory, Outcome } from "@armada/protocol";
import type { BayHistoryRead } from "../shared/api/bay-history";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** The query string for one bay: both asked for, the repository encoded. */
export function slotHistoryPath(manifestId: string, slot: number): string {
  return `/slot_history?manifest_id=${encodeURIComponent(manifestId)}&slot=${slot}`;
}

export class BayHistoryHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(manifestId: string, slot: number): Promise<BayHistoryRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", slotHistoryPath(manifestId, slot));
    return answer.ok === true ? { ok: true, value: answer.body as BayHistory } : { ok: false, outcome: answer.outcome };
  }
}
