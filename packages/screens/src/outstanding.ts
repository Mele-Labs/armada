// A question held open for a person. Its own file, without JSX, because main publishes
// it on `BridgeState` and main's build reads no `.tsx`.

import type { HelmCallInFlight } from "@armada/protocol";

/**
 * One question held open for a person: a call Helm made that the person's own settings do not
 * cover, held open while they answer. **Wire shape whole**, so the answer comes from what was asked
 * rather than from what a card drew. #1389.
 */
export type Outstanding = { kind: "helm"; call: HelmCallInFlight };

/** Unique across the list. */
export function outstandingId(question: Outstanding): string {
  return `helm:${question.call.call}`;
}
