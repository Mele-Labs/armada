// The one function a waiting item is answered through: `window.armada.answerWaiting`, Fleet's
// `POST /sessions/waiting/answer`. A `choice` is the index of one of the item's options, `text` is the
// person's own words, and `mode` hands the decision to the agent. What it came to is told in one of
// three ways, so the card can act on it.

import { refusalWords } from "@armada/screens/src/refusal-words";
import type { AnswerWaiting } from "@armada/protocol";

/** Fleet's refusal for an item nothing holds (409): it was settled already, or the Session stopped waiting. */
const UNHELD = "fleet.session_waiting_unheld";

/** Answered, or nothing held it any more (the card goes quietly), or Fleet refused in these words. */
export type WaitingAnswered = { kind: "answered" } | { kind: "gone" } | { kind: "refused"; said: string };

export async function answerWaiting(request: AnswerWaiting): Promise<WaitingAnswered> {
  const done = await window.armada.answerWaiting(request);
  if (done.ok) return { kind: "answered" };
  const outcome = done.outcome;
  if (!outcome.ok && outcome.why === "refused" && outcome.error.code === UNHELD) return { kind: "gone" };
  return { kind: "refused", said: refusalWords(outcome) };
}
