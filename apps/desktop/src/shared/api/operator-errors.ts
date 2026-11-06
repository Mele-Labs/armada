// An error Bridge showed the person, handed to Fleet's Operator. One route; nothing comes back that a window draws.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { BridgeErrorTaken, Outcome, ReportBridgeError } from "@armada/protocol";

/** What the send came to. The caller swallows a refusal: a failed report is never a failure card of its own. */
export type OperatorErrorSent =
  | {
      ok: true;
      value: BridgeErrorTaken;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

export type OperatorErrorsApi = {
  /** `report_bridge_error` (`POST /operator/errors`): the facts of a failure card, once, when it was raised. */
  reportBridgeError: (error: ReportBridgeError) => Promise<OperatorErrorSent>;
};

export const OPERATOR_ERRORS_CHANNELS = {
  reportBridgeError: "bridge:report-bridge-error",
} as const;
