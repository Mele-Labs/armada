// Helm's dock: the repository it answers for, its held calls and the questions waiting on a person.
// Its thread is the Helm Session's, which the window holds with every other Session.
// A slice imports protocol and screens, never another slice; desktop's `shared/api.ts` composes them.

import type {
  HelmCallAnswer,
  Outcome,
  HelmContext,
  HelmDebugRead,
  SessionTag,
  SessionUpload,
  ThoughtRead,
  ThoughtTarget,
} from "@armada/protocol";
import type { Outstanding } from "@armada/screens/src/outstanding";

/**
 * What a message to Helm carries beside its words: files and pictures, and what it names with `@` and `#`.
 * `model` and `effort` are the box's pick, read only by the message that starts the Helm Session; a live one
 * is tuned as any Session is.
 */
export type HelmCarry = { attachments?: SessionUpload[]; mentions?: SessionTag[]; model?: string; effort?: string };

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
   * for. The first message starts that repository's Helm Session and the reply is its thread, which
   * the window holds as it holds every Session's. `context` names where the person is.
   * `carry` is what the message went with; **the text may be blank when it
   * carries something**, and a message with neither is not sent.
   */
  askHelm: (text: string, context?: HelmContext, carry?: HelmCarry) => Promise<Outcome>;
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
   * Said after the words were delivered to a Session or a Job: Fleet keeps a `sent` row for them in the
   * thread of `manifestId`'s Helm Session, and it arrives as any row of a Session's thread does.
   * **Never sent before the delivery succeeded**, and never to Helm's own session. `carry` is what the
   * thought went with, which Fleet keeps on the row so a reload draws it.
   */
  recordThoughtSent: (manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry) => Promise<Outcome>;
  /** End the Helm Session of the repository Helm answers for, so the next message starts a new one. */
  startHelmFresh: () => Promise<Outcome>;
  /**
   * "Discuss with Helm" on a card, or the dock's own switch on All
   * repositories. **The rail's own pick does not move.** A pick still wins
   * once made — this is only ever the tie-break for All.
   */
  pointHelm: (manifestId: string) => Promise<void>;
};

/** The repository Helm answers for. Its Session is one of the window's Sessions, found by its `helm` mark and this id. */
export type HelmAim = { state: "none" } | { state: "pointed"; manifestId: string };

export type HelmState = {
  /**
   * Every question waiting on a person — a Drone's, a held command, a Judge refusal — from every
   * repository Fleet serves, **whatever the rail picked**. Helm's dock draws them. `questions.ts`.
   */
  questions: Outstanding[];
  /** Which repository Helm answers for is main's own decision; see `main/helm.ts`. */
  helm: HelmAim;
};

export const HELM_NOTHING_YET: HelmState = {
  questions: [],
  helm: { state: "none" },
};

export const HELM_CHANNELS = {
  // One call helm was held on, answered in the dock. No job id: it names the
  // call fleet minted, and nothing on the board moves. #1389.
  answerHelmCall: "bridge:answer-helm-call",
  // Helm's conversation: say something, end its Session, and point it at a
  // repository without moving the rail's own pick. #944.
  askHelm: "bridge:ask-helm",
  // The session as one record, read once when a person opens it — #1367.
  helmDebugInfo: "bridge:helm-debug-info",
  // Helm reading a thought for where it belongs, and recording where it went.
  readThought: "bridge:read-thought",
  recordThoughtSent: "bridge:record-thought-sent",
  startHelmFresh: "bridge:start-helm-fresh",
  pointHelm: "bridge:point-helm",
} as const;
