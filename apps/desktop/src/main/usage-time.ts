// Where Jobs' time went, as Fleet answers `GET /usage/time`, `GET /usage/hours` and `GET /sessions/time`. The window is Fleet's
// own (seven days, 48 hours), so only a Job's id is ever asked. Nothing is kept here; the renderer decides when to ask again.

import type { JobTime, Outcome, UsageHours, UsageTime } from "@armada/protocol";
import type { JobTimeRead, UsageHoursRead, UsageTimeRead } from "../shared/api/usage-time";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = {
  ok: false,
  why: "not_connected",
};

export const USAGE_TIME_PATH = "/usage/time";
export const USAGE_HOURS_PATH = "/usage/hours";

/** The path for one Job, its id encoded. */
export function jobTimePath(sessionId: string): string {
  return `/sessions/time?session_id=${encodeURIComponent(sessionId)}`;
}

type Read<T> =
  | {
      ok: true;
      value: T;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

export class UsageTimeHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  time(): Promise<UsageTimeRead> {
    return this.read<UsageTime>(USAGE_TIME_PATH);
  }

  hours(): Promise<UsageHoursRead> {
    return this.read<UsageHours>(USAGE_HOURS_PATH);
  }

  job(sessionId: string): Promise<JobTimeRead> {
    return this.read<JobTime>(jobTimePath(sessionId));
  }

  private async read<T>(path: string): Promise<Read<T>> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "GET", path);
    return answer.ok === true
      ? {
          ok: true,
          value: answer.body as T,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }
}
