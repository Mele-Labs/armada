// Helm's dock: its thread, its held calls and the questions waiting on a person.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  HelmCallAnswer,
  Outcome,
  HelmContext,
  HelmDebugRead,
  HelmThread,
} from "@armada/protocol";
import type { Outstanding } from "@armada/screens/src/outstanding";

export type HelmApi = {
  /**
   * Answer one held helm call: allow it once, allow it and write the rule into
   * this repository's own personal agent settings, or refuse it. #1389.
   *
   * **The session is waiting inside its own tool call while this is out**, so
   * an answer that lands is what it runs on. A 409 where nothing is waiting
   * under that call — it was answered already, or fleet's hold ran out.
   */
  answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string) => Promise<Outcome>;
  /**
   * Say something to Helm, about whichever repository it currently answers
   * for. The reply arrives on `BridgeState.helm`. `context` names where the
   * person is — omitted, an older Bridge's shape, Helm asks as it always has.
   */
  askHelm: (text: string, context?: HelmContext) => Promise<Outcome>;
  /**
   * The Helm session Bridge is pointed at, as one quotable record — #1367.
   *
   * **`readCheckOutput`'s shape, for its reasons.** It is one person's
   * gesture on one conversation, answered once and never republished: a record
   * in `BridgeState` would redraw every surface each time a reply arrived.
   */
  helmDebugInfo: () => Promise<HelmDebugRead>;
  /** Forget Helm's stored session and the thread. Refused while a reply is being written. */
  startHelmFresh: () => Promise<Outcome>;
  /**
   * "Discuss with Helm" on a card, or the dock's own switch on All
   * repositories. **The rail's own pick does not move.** A pick still wins
   * once made — this is only ever the tie-break for All.
   */
  pointHelm: (manifestId: string) => Promise<void>;
};

export type HelmState = {
  /**
   * Every question waiting on a person — a Drone's, a held command, a Judge refusal — from every
   * repository Fleet serves, **whatever the rail picked**. Helm's dock draws them. `questions.ts`.
   */
  questions: Outstanding[];
  /**
   * One repository's Helm conversation — the dock's own thread, under the
   * questions above. Which repository it answers for is main's own decision;
   * see `main/helm.ts`.
   */
  helm: HelmThread;
};

export const HELM_NOTHING_YET: HelmState = {
  questions: [],
  helm: { state: "none" },
};

export const HELM_CHANNELS = {
  // One call helm was held on, answered in the dock. No job id: it names the
  // call fleet minted, and nothing on the board moves. #1389.
  answerHelmCall: "bridge:answer-helm-call",
  // Helm's conversation: say something, forget it, and point it at a
  // repository without moving the rail's own pick. #944.
  askHelm: "bridge:ask-helm",
  // The session as one record, read once when a person opens it — #1367.
  helmDebugInfo: "bridge:helm-debug-info",
  startHelmFresh: "bridge:start-helm-fresh",
  pointHelm: "bridge:point-helm",
} as const;
