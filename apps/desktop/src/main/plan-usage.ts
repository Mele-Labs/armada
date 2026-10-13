// The plan's windows, as main holds them: one request to Fleet and the stream's `plan_usage.changed`,
// passed on to every window. Nothing is kept here, since the reading arrives whole.

import type { Outcome, PlanUsage } from "@armada/protocol";
import type { PlanUsageRead } from "../shared/api/plan-usage";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class PlanUsageHost {
  private readonly port: () => number | null;
  private told: (usage: PlanUsage) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed reading goes: main's, which owns the windows. */
  onChanged(told: (usage: PlanUsage) => void): void {
    this.told = told;
  }

  /** `plan_usage.changed`. */
  changed(usage: PlanUsage): void {
    this.told(usage);
  }

  async get(): Promise<PlanUsageRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", "/plan_usage");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as PlanUsage };
  }
}
