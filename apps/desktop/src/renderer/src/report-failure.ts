// Each failure card Bridge draws, sent to Fleet's Operator once, with no press (`report_bridge_error`, `POST /operator/errors`).
//
// **A send is never a failure of its own.** It goes through `window.armada` and not through the commands that raise a toast,
// its answer is dropped, and a rejection is caught here: an unhandled one would reach `watchUncaught`, draw a card, and send
// that card, round again. A card raised while Fleet is down is simply not taken.
//
// **The same card in a few seconds is sent once.** Fleet de-duplicates by day; this saves it a flood from a render that
// raises on every pass.

import type { ReportBridgeError } from "@armada/protocol";
import type { FailureFacts } from "@armada/shell";
import { watchFailureShown } from "@armada/shell";

import type { OperatorErrorSent } from "../../shared/api/operator-errors";

/** How long the same code and message stay one card. */
export const SAME_CARD_MS = 5_000;

/** The fields whose values are log paths the card names: the restart log, and Bridge's own `audit.jsonl`. */
const LOG_KEYS = [
  "log",
  "bridge_log",
];

/** What `report_bridge_error` is told about a card. */
export function reportedError(facts: FailureFacts, at: Date): ReportBridgeError {
  const fields = facts.fields ?? [];
  const logs = fields.filter(({ key }) => LOG_KEYS.includes(key)).map(({ value }) => value);
  const commit = fields.find(({ key }) => key === "commit")?.value;
  return {
    code: facts.code,
    message: facts.message,
    ...(fields.length === 0
      ? {}
      : {
          fields: fields.map(({ key, value }) => ({
            key,
            value,
          })),
        }),
    ...(facts.chain === undefined || facts.chain.length === 0
      ? {}
      : {
          chain: facts.chain,
        }),
    ...(logs.length === 0
      ? {}
      : {
          logs,
        }),
    ...(commit === undefined
      ? {}
      : {
          commit,
        }),
    ...(facts.job_id === undefined
      ? {}
      : {
          job_id: facts.job_id,
        }),
    at: at.toISOString(),
  };
}

/** The send, as the preload offers it. */
type Send = (error: ReportBridgeError) => Promise<OperatorErrorSent>;

/** What each card is passed to: the dedupe in front of one send. */
export function failureReporter(send: Send, now: () => Date = () => new Date()) {
  const lately = new Map<string, number>();
  return (facts: FailureFacts): void => {
    const at = now();
    for (const [key, sentAt] of lately)
      if (at.getTime() - sentAt >= SAME_CARD_MS) lately.delete(key);
    const key = `${facts.code}\u0000${facts.message}`;
    if (lately.has(key)) return;
    lately.set(key, at.getTime());
    try {
      // The answer is Fleet's to keep. Nothing here draws it, and nothing here throws from it.
      send(reportedError(facts, at)).catch((cause: unknown) =>
        console.debug("report_bridge_error", cause),
      );
    } catch (cause) {
      console.debug("report_bridge_error", cause);
    }
  };
}

/** Pass every failure card on to the Operator, from now until the returned function is called. */
export function wireFailureReports(send: Send): () => void {
  return watchFailureShown(failureReporter(send));
}
