// Telling somebody the agents' account has signed out, when Bridge may not be in front of them.
//
// **A notice is a signed-out spell beginning, once for each spell.** A spell is named by its
// `signed_out_since`, so the press that starts the sign-in, a failed attempt, and a replayed event all
// carry the spell already told and say nothing. The state Bridge finds on connecting is where things
// stand and is not news, which is `downshift-notice.ts`'s seeding for the same reason.
//
// **The tone work that has stopped takes.** Every Job on the account is stopped until it is signed in,
// and that is `blocked`, never the quieter `waiting`. The effect arrives as a function, like every seam in
// `src/main`, so this runs in node with no Electron in it.

import type { AgentSignIn } from "@armada/protocol";

import type { Notice, NoticeEffects } from "./downshift-notice";
import { soundOf } from "./telling";

export class AgentSignInNotice {
  private readonly effects: NoticeEffects;
  private told: string | undefined;

  constructor(effects: NoticeEffects) {
    this.effects = effects;
  }

  /** The state as it stands on connecting: remembered, and told to nobody. */
  seed(state: AgentSignIn): void {
    this.told = state.signed_out_since;
  }

  /** `agent_sign_in.changed`. */
  read(state: AgentSignIn): void {
    const since = state.signed_out_since;
    if (since === undefined || since === this.told) return;
    this.told = since;
    const notice: Notice = { title: "Claude is signed out", body: "", sound: soundOf("blocked") };
    this.effects.show(notice);
  }
}
