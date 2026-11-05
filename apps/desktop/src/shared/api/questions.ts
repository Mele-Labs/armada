import type { CallAnswer, Outcome } from "@armada/protocol";
import type { Outstanding } from "@armada/screens/src/outstanding";

export type QuestionsApi = {
  /**
   * Answer one held call: allow it once, allow it and write the rule into
   * this repository's own personal agent settings, or refuse it. #1389.
   *
   * **The session is waiting inside its own tool call while this is out**, so
   * an answer that lands is what it runs on. A 409 where nothing is waiting
   * under that call — it was answered already, or fleet's hold ran out.
   */
  answerCall: (call: string, answer: CallAnswer, note?: string) => Promise<Outcome>;
};

export type QuestionsState = {
  /**
   * Every call held open for a person from every repository Fleet serves, **whatever the rail
   * picked**. `questions.ts`.
   */
  questions: Outstanding[];
};

export const QUESTIONS_NOTHING_YET: QuestionsState = {
  questions: [],
};

export const QUESTIONS_CHANNELS = {
  answerCall: "bridge:answer-call",
} as const;
