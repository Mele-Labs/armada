// Telling somebody the default model is offered a step down, or due back, when they are not looking at Bridge.
//
// **A notice is the state entering `offered` or `reset`, once for each entry.** A percent that moves while
// the offer stands is the same offer, so nothing is told for it; the weekly window resetting is a second
// thing and gets its own. The state Bridge finds on connecting is where things stand and is not news, which
// is `telling.ts`'s seeding for the same reason.
//
// **The quietest tone that still interrupts.** The offer is a decision that will keep, so it takes the tone a
// job waiting for review takes and never the one for work that has stopped; and it goes no further than the
// OS, since nothing here is an escalation. `design-system.md`, *Sound*. The effect arrives as a function,
// like every seam in `src/main`, so this runs in node with no Electron in it.

import type { ModelDownshiftState } from "@armada/protocol";

import { soundOf } from "./telling";

/** What the shell posts. The body is the fact, and the title is the act it offers. */
export type Notice = { title: string; body: string; sound: string };

export type NoticeEffects = {
  /** Post one notification, and bring Bridge forward when it is pressed. */
  show: (notice: Notice) => void;
};

type Standing = "offered" | "shifted" | "reset" | undefined;

export class DownshiftNotice {
  private readonly effects: NoticeEffects;
  private standing: Standing;

  constructor(effects: NoticeEffects) {
    this.effects = effects;
  }

  /** The state as it stands on connecting: remembered, and told to nobody. */
  seed(state: ModelDownshiftState): void {
    this.standing = state.downshift?.phase;
  }

  /** `model_downshift.changed`. */
  read(state: ModelDownshiftState): void {
    const next = state.downshift;
    const entered = next?.phase !== this.standing;
    this.standing = next?.phase;
    if (!entered || next === undefined) return;
    if (next.phase === "offered") {
      this.effects.show({
        title: `Default to ${next.to} offered`,
        body: `Weekly limit ${Math.round(next.percent)}% used`,
        sound: soundOf("waiting"),
      });
    } else if (next.phase === "reset") {
      this.effects.show({ title: `Back to ${next.was} offered`, body: "Weekly limit reset", sound: soundOf("waiting") });
    }
  }
}
