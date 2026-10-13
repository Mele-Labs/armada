// The default model's step down: the state Fleet holds, the four answers to it, and the event that
// carries it whole. A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { ModelDownshiftState, Outcome } from "@armada/protocol";

/** What reading or answering the offer came to: the state as Fleet holds it now, or the refusal to word. */
export type ModelDownshiftRead = { ok: true; value: ModelDownshiftState } | { ok: false; outcome: Outcome };

export type ModelDownshiftApi = {
  /** Whether the default model is offered a step down, stepped down, or due back. */
  getModelDownshift: () => Promise<ModelDownshiftRead>;
  /** Valid while offered: the default is the lower model until the weekly window resets. */
  acceptDownshift: () => Promise<ModelDownshiftRead>;
  /** Valid while stepped down or reset: the default is what it was. */
  restoreDownshift: () => Promise<ModelDownshiftRead>;
  /** Valid once the window has reset: the lower model stays the default. */
  keepDownshift: () => Promise<ModelDownshiftRead>;
  /** Valid while offered: the owner is choosing by hand, and no second offer comes in this window. */
  settleDownshift: () => Promise<ModelDownshiftRead>;
  /** `model_downshift.changed`: the state, whole, whenever Fleet moves it. */
  onModelDownshiftChanged: (onChanged: (state: ModelDownshiftState) => void) => () => void;
};

export const MODEL_DOWNSHIFT_CHANNELS = {
  getModelDownshift: "bridge:get-model-downshift",
  acceptDownshift: "bridge:accept-downshift",
  restoreDownshift: "bridge:restore-downshift",
  keepDownshift: "bridge:keep-downshift",
  settleDownshift: "bridge:settle-downshift",
  /** Main to every window, not a request. */
  modelDownshiftChanged: "bridge:model-downshift-changed",
} as const;
