// Sleep mode: the switch, the night it keeps, and a correction of one decision. A slice imports protocol,
// never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { Outcome, SleepState } from "@armada/protocol";

/** What a sleep act came to: the night as Fleet answered with it, or the refusal to word. */
export type SleepActed = { ok: true; value: SleepState } | { ok: false; outcome: Outcome };

export type SleepApi = {
  /** The switch and the night's rows. */
  getSleep: () => Promise<SleepActed>;
  /** On starts a new night; off keeps it for the Morning review. */
  setSleep: (on: boolean) => Promise<SleepActed>;
  /** Tells the session what the owner would have said, and marks the decision corrected. */
  overrideSleep: (id: string, text: string) => Promise<SleepActed>;
  /** `sleep.changed`: the night, whole, whenever Fleet changes it. */
  onSleepChanged: (onChanged: (state: SleepState) => void) => () => void;
};

export const SLEEP_CHANNELS = {
  getSleep: "bridge:get-sleep",
  setSleep: "bridge:set-sleep",
  overrideSleep: "bridge:override-sleep",
  /** Main to every window, not a request. */
  sleepChanged: "bridge:sleep-changed",
} as const;
