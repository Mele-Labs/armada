// What the last seven days cost, as Fleet answers `GET /usage/summary`: one request, the offset of the caller's local time
// and, for a build's tokens, the stretch it ran. Nothing is kept here; Fleet sums, and the renderer decides when to ask again.

import type { Outcome, UsageRules, UsageSpentUnits, UsageSummary } from "@armada/protocol";
import type {
  UsageRulesRead,
  UsageSpentRead,
  UsageSummaryRead,
  UsageWindow,
} from "../shared/api/usage-summary";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = {
  ok: false,
  why: "not_connected",
};

/** The path for a caller `tzOffsetMinutes` ahead of UTC; with a `window`, the turns in `[since, until)` and not the last seven days. */
export function usageSummaryPath(tzOffsetMinutes: number, window?: UsageWindow): string {
  const zone = `/usage/summary?tz_offset_minutes=${tzOffsetMinutes}`;
  return window === undefined
    ? zone
    : `${zone}&since=${encodeURIComponent(window.since)}&until=${encodeURIComponent(window.until)}`;
}

export class UsageSummaryHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async get(tzOffsetMinutes: number, window?: UsageWindow): Promise<UsageSummaryRead> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "GET", usageSummaryPath(tzOffsetMinutes, window));
    return answer.ok === true
      ? {
          ok: true,
          value: answer.body as UsageSummary,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }

  /** `GET /usage/rules`, with Fleet's own window of the last seven days. */
  async rules(): Promise<UsageRulesRead> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "GET", "/usage/rules");
    return answer.ok === true
      ? {
          ok: true,
          value: answer.body as UsageRules,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }

  /** `GET /usage/spent`, with Fleet's own windows up to now. */
  async spent(): Promise<UsageSpentRead> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "GET", "/usage/spent");
    return answer.ok === true
      ? {
          ok: true,
          value: answer.body as UsageSpentUnits,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }
}
