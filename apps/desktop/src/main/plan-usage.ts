// The plan's windows, as main holds them: one request to Fleet and the stream's `plan_usage.changed`,
// passed on to every window. The newest reading is held, and kept on disk by `remembered.ts`, so a window opened
// before Fleet answers draws the last one.

import type { Outcome, PlanUsage } from "@armada/protocol";
import type { PlanUsageRead } from "../shared/api/plan-usage";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class PlanUsageHost {
  private readonly port: () => number | null;
  private told: (usage: PlanUsage) => void = () => {};
  private held: PlanUsage | null = null;
  private keep: (usage: PlanUsage) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed reading goes: main's, which owns the windows. */
  onChanged(told: (usage: PlanUsage) => void): void {
    this.told = told;
  }

  /** Start from the reading on disk, and say where each newer one is kept. */
  remembering(held: PlanUsage | null, keep: (usage: PlanUsage) => void): void {
    this.held = held;
    this.keep = keep;
  }

  /** `plan_usage.changed`. */
  changed(usage: PlanUsage): void {
    this.held = usage;
    this.keep(usage);
    this.told(usage);
  }

  async get(): Promise<PlanUsageRead> {
    const port = this.port();
    if (port === null) return this.held === null ? { ok: false, outcome: NOT_CONNECTED } : { ok: true, value: this.held };
    const answer = await ask(port, "GET", "/plan_usage");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    this.held = answer.body as PlanUsage;
    this.keep(this.held);
    return { ok: true, value: this.held };
  }
}
