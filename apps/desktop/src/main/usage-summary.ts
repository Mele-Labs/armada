// What the last seven days cost, as Fleet answers `GET /usage/summary`: one request, the offset of the caller's local time
// the only thing asked. Nothing is kept here; Fleet sums, and the renderer decides when to ask again.

import type { Outcome, UsageSummary } from "@armada/protocol";
import type { UsageSummaryRead } from "../shared/api/usage-summary";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** The path for a caller `tzOffsetMinutes` ahead of UTC. */
export function usageSummaryPath(tzOffsetMinutes: number): string {
  return `/usage/summary?tz_offset_minutes=${tzOffsetMinutes}`;
}

export class UsageSummaryHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(tzOffsetMinutes: number): Promise<UsageSummaryRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", usageSummaryPath(tzOffsetMinutes));
    return answer.ok === true ? { ok: true, value: answer.body as UsageSummary } : { ok: false, outcome: answer.outcome };
  }
}
