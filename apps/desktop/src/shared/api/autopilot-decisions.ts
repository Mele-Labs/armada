// Autopilot's answers: what it decided while the owner was away, and a correction of one. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { AutopilotDecisions, Outcome } from "@armada/protocol";

/** What an act on the answers came to: the list as Fleet answered with it, or the refusal to word. */
export type AutopilotDecisionsActed = { ok: true; value: AutopilotDecisions } | { ok: false; outcome: Outcome };

export type AutopilotDecisionsApi = {
  /** Every repository's answers not yet reviewed, in the order they were given. */
  getAutopilotDecisions: () => Promise<AutopilotDecisionsActed>;
  /** Keeps an answer as it was: it leaves the list. */
  reviewAutopilotDecision: (id: string) => Promise<AutopilotDecisionsActed>;
  /** Sends the Job what the owner would have said, and marks the answer corrected and reviewed. */
  overrideAutopilotDecision: (id: string, text: string) => Promise<AutopilotDecisionsActed>;
  /** `autopilot.decisions_changed`: the list, whole, whenever Fleet changes it. */
  onAutopilotDecisionsChanged: (onChanged: (decisions: AutopilotDecisions) => void) => () => void;
};

export const AUTOPILOT_DECISIONS_CHANNELS = {
  getAutopilotDecisions: "bridge:get-autopilot-decisions",
  reviewAutopilotDecision: "bridge:review-autopilot-decision",
  overrideAutopilotDecision: "bridge:override-autopilot-decision",
  /** Main to every window, not a request. */
  autopilotDecisionsChanged: "bridge:autopilot-decisions-changed",
} as const;
