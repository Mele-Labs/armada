// The default model's step down, as main holds it: a read and four acts against Fleet, and the stream's
// `model_downshift.changed`, passed on to every window. Nothing is kept here, since the state arrives whole.

import type { ModelDownshiftState, Outcome } from "@armada/protocol";
import type { ModelDownshiftRead } from "../shared/api/model-downshift";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class ModelDownshiftHost {
  private readonly port: () => number | null;
  private told: (state: ModelDownshiftState) => void = () => {};
  private seen: (state: ModelDownshiftState) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed state goes: main's, which owns the windows and the notification. */
  onChanged(told: (state: ModelDownshiftState) => void): void {
    this.told = told;
  }

  /** Say where the state read on connecting goes: it is where things stand, not news. */
  onSeeded(seen: (state: ModelDownshiftState) => void): void {
    this.seen = seen;
  }

  /** `model_downshift.changed`. */
  changed(state: ModelDownshiftState): void {
    this.told(state);
  }

  /** Read once on a connection, for nobody: a state already standing is not news to the notification. */
  async seed(): Promise<void> {
    const read = await this.get();
    if (read.ok) this.seen(read.value);
  }

  async get(): Promise<ModelDownshiftRead> {
    return await this.act("GET", "/model_downshift");
  }

  async accept(): Promise<ModelDownshiftRead> {
    return await this.act("POST", "/model_downshift/accept");
  }

  async restore(): Promise<ModelDownshiftRead> {
    return await this.act("POST", "/model_downshift/restore");
  }

  async keep(): Promise<ModelDownshiftRead> {
    return await this.act("POST", "/model_downshift/keep");
  }

  async settle(): Promise<ModelDownshiftRead> {
    return await this.act("POST", "/model_downshift/settle");
  }

  private async act(method: "GET" | "POST", path: string): Promise<ModelDownshiftRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as ModelDownshiftState };
  }
}
