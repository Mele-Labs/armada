// Helm's dock: its thread, its held calls and the questions waiting on a person.
// A slice imports protocol and screens, never another slice; desktop's `shared/api.ts` composes them.

import type {
  FrameRead,
  HelmBoxFacts,
  HelmCallAnswer,
  Outcome,
  HelmContext,
  HelmDebugRead,
  HelmThread,
  SessionTag,
  SessionUpload,
  ThoughtRead,
  ThoughtTarget,
} from "@armada/protocol";
import type { Outstanding } from "@armada/screens/src/outstanding";

/** What a message to Helm carries beside its words: files and pictures, and what it names with `@` and `#`. */
export type HelmCarry = { attachments?: SessionUpload[]; mentions?: SessionTag[] };

/** What `tuneHelm` came to: the facts Fleet now holds for the conversation, or the refusal to word. */
export type HelmTuned = { ok: true; facts: HelmBoxFacts } | { ok: false; outcome: Outcome };

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
   * `carry` is what the message went with; **the text may be blank when it
   * carries something**, and a message with neither is not sent.
   */
  askHelm: (text: string, context?: HelmContext, carry?: HelmCarry) => Promise<Outcome>;
  /**
   * The model and effort Helm uses for a conversation, `null` for Auto. `manifestId` names the
   * conversation; absent, the one Helm points at. The facts Fleet now holds come back, and
   * `BridgeState.helm` carries them too.
   */
  tuneHelm: (manifestId: string | undefined, model: string | null, effort: string | null) => Promise<HelmTuned>;
  /** A picture or file a message to Helm carried, by the id its row gave. */
  readHelmFile: (manifestId: string, file: string) => Promise<FrameRead>;
  /**
   * The Helm session Bridge is pointed at, as one quotable record — #1367.
   *
   * **`readCheckOutput`'s shape, for its reasons.** It is one person's
   * gesture on one conversation, answered once and never republished: a record
   * in `BridgeState` would redraw every surface each time a reply arrived.
   */
  helmDebugInfo: () => Promise<HelmDebugRead>;
  /**
   * What Helm makes of words typed into its box: a question for itself, or a thought for other
   * running work, with the best match if any. **`helmDebugInfo`'s shape, for its reason**: asked
   * once by the dock and the dock's alone. A read that failed is the dock's cue to ask Helm, so
   * the words are never lost.
   */
  readThought: (text: string) => Promise<ThoughtRead>;
  /**
   * Said after the words were delivered to a Session or a Job: Fleet keeps a row for them in the
   * thread of `manifestId`, the conversation the dock shows, and it arrives on `BridgeState.helm`.
   * **Never sent before the delivery succeeded**, and never to Helm's own session.
   */
  recordThoughtSent: (manifestId: string, text: string, target: ThoughtTarget) => Promise<Outcome>;
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
  // The model and effort Helm uses, and a file one of its messages carried.
  tuneHelm: "bridge:tune-helm",
  readHelmFile: "bridge:read-helm-file",
  // The session as one record, read once when a person opens it — #1367.
  helmDebugInfo: "bridge:helm-debug-info",
  // Helm reading a thought for where it belongs, and recording where it went.
  readThought: "bridge:read-thought",
  recordThoughtSent: "bridge:record-thought-sent",
  startHelmFresh: "bridge:start-helm-fresh",
  pointHelm: "bridge:point-helm",
} as const;
