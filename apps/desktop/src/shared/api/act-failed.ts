// A background act that failed after its request was answered: the event that carries the refusal.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { ActFailed } from "@armada/protocol";

export type ActFailedApi = {
  /** `act.failed`: the act, what it was for, and the refusal Fleet had no answer left to carry. */
  onActFailed: (onFailed: (failed: ActFailed) => void) => () => void;
};

export const ACT_FAILED_CHANNELS = {
  /** Main to every window, not a request. */
  actFailed: "bridge:act-failed",
} as const;
