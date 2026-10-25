// The agents' sign-in, as main holds it: a read and one press against Fleet, and the stream's
// `agent_sign_in.changed`, passed on to every window. Nothing is kept here, since the state arrives whole.

import type { AgentSignIn, Outcome } from "@armada/protocol";
import type { AgentSignInRead } from "../shared/api/agent-sign-in";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class AgentSignInHost {
  private readonly port: () => number | null;
  private told: (state: AgentSignIn) => void = () => {};
  private seen: (state: AgentSignIn) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where a changed state goes: main's, which owns the windows and the notification. */
  onChanged(told: (state: AgentSignIn) => void): void {
    this.told = told;
  }

  /** Say where the state read on connecting goes: it is where things stand, not news. */
  onSeeded(seen: (state: AgentSignIn) => void): void {
    this.seen = seen;
  }

  /** `agent_sign_in.changed`. */
  changed(state: AgentSignIn): void {
    this.told(state);
  }

  /** Read once on a connection, for nobody: a spell already standing is not news to the notification. */
  async seed(): Promise<void> {
    const read = await this.get();
    if (read.ok) this.seen(read.value);
  }

  async get(): Promise<AgentSignInRead> {
    return await this.act("GET");
  }

  async start(): Promise<AgentSignInRead> {
    return await this.act("POST");
  }

  private async act(method: "GET" | "POST"): Promise<AgentSignInRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, "/agent/sign_in");
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as AgentSignIn };
  }
}
