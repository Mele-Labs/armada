// Every Helm call waiting on a person — #935, #1389.
//
// **Folded where the event carries the call, read where it does not.** `helm.asking_to_run` carries
// the call whole and `helm.call_answered` takes it down. A resync reads `/helm/calls`: events arrive
// only for what happens next, so a Bridge started while a call was waiting would list nothing.

import type { HelmCallInFlight, HelmCallsWaiting } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";
import { ask } from "./request";

export type QuestionsWiring = {
  current: () => BridgeState;
  publish: (change: Partial<BridgeState>) => void;
};

export class Questions {
  private readonly wiring: QuestionsWiring;

  constructor(wiring: QuestionsWiring) {
    this.wiring = wiring;
  }

  /**
   * `helm.asking_to_run`. **Replaces the ask under the same call id**, so a
   * re-delivered event draws one card and not two. #1389.
   */
  helmAsking(waiting: HelmCallInFlight): void {
    const held = this.wiring.current().questions;
    const others = held.filter((question) => question.call.call !== waiting.call);
    this.wiring.publish({ questions: [...others, { kind: "helm", call: waiting }] });
  }

  /** `helm.call_answered`, and the answer a person's own press came back with. */
  helmAnswered(call: string): void {
    const held = this.wiring.current().questions;
    const kept = held.filter((question) => question.call.call !== call);
    if (kept.length !== held.length) this.wiring.publish({ questions: kept });
  }

  /** Every Helm call waiting, on a resync or a Refresh. A failed read keeps what is held. */
  async readHelmCalls(port: number): Promise<void> {
    const answer = await ask(port, "GET", "/helm/calls");
    if (answer.ok !== true) return;
    const { waiting } = answer.body as HelmCallsWaiting;
    // A body that is not a list is a fleet that does not serve this yet. Held
    // calls stay as they are rather than being cleared by a 404's shape.
    if (!Array.isArray(waiting)) return;
    this.wiring.publish({ questions: waiting.map((call) => ({ kind: "helm" as const, call })) });
  }
}
