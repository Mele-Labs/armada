// Autopilot's answers, as main holds them: three requests to Fleet and the stream's
// `autopilot.decisions_changed`, passed on to every window. Nothing is kept here, since the list lives
// in Fleet's store and arrives whole.

import type { AutopilotDecisions, Outcome } from "@armada/protocol";
import type { AutopilotDecisionsActed } from "../shared/api/autopilot-decisions";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class AutopilotDecisionsHost {
  private readonly port: () => number | null;
  private told: (decisions: AutopilotDecisions) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed list goes: main's, which owns the windows. */
  onChanged(told: (decisions: AutopilotDecisions) => void): void {
    this.told = told;
  }

  /** `autopilot.decisions_changed`. */
  changed(decisions: AutopilotDecisions): void {
    this.told(decisions);
  }

  async get(): Promise<AutopilotDecisionsActed> {
    return await this.act("GET", "/autopilot/decisions");
  }

  async override(id: string, text: string): Promise<AutopilotDecisionsActed> {
    return await this.act("POST", "/autopilot/decisions/override", { id, text });
  }

  async review(id: string): Promise<AutopilotDecisionsActed> {
    return await this.act("POST", "/autopilot/decisions/review", { id });
  }

  private async act(method: "GET" | "POST", path: string, body?: unknown): Promise<AutopilotDecisionsActed> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as AutopilotDecisions };
  }
}
