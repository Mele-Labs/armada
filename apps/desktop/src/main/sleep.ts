// Sleep mode, as main holds it: three requests to Fleet and the stream's `sleep.changed`, passed on to
// every window. Nothing is kept here, since the night lives in Fleet's store and arrives whole.

import type { Outcome, SleepState } from "@armada/protocol";
import type { SleepActed } from "../shared/api/sleep";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class SleepHost {
  private readonly port: () => number | null;
  private told: (state: SleepState) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed night goes: main's, which owns the windows. */
  onChanged(told: (state: SleepState) => void): void {
    this.told = told;
  }

  /** `sleep.changed`. */
  changed(state: SleepState): void {
    this.told(state);
  }

  async get(): Promise<SleepActed> {
    return await this.act("GET", "/sleep");
  }

  async set(on: boolean): Promise<SleepActed> {
    return await this.act("POST", "/sleep", { on });
  }

  async override(id: string, text: string): Promise<SleepActed> {
    return await this.act("POST", "/sleep/override", { id, text });
  }

  private async act(method: "GET" | "POST", path: string, body?: unknown): Promise<SleepActed> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as SleepState };
  }
}
