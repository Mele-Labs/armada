// Each turn's reasoning is a folded row; the reasoning before a question asked with nothing written to him since his
// message is open above it. Over the `reasoning-before-a-question` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { deckRow, inside, role, text, walk } from "../walk";

const { thread } = kit(false);
const needsYou = role("button", "Needs you");

const reasoningBeforeAQuestion = walk("reasoning-before-a-question", [
  toSessions,
  {
    press: deckRow("Native Jobs screens"),
    say: "The Job that asked about Queue for later",
  },
  {
    press: needsYou,
    say: "Fold the Needs you panel so the thread shows",
  },
  {
    look: inside(thread, text(/Native has no Cockpit to list saved ideas/)),
    say: "It wrote you nothing before its question, so the reasoning behind it is open",
  },
  {
    look: inside(thread, text(/Thought for 7s/)),
    say: "Earlier, it answered in words, so that reasoning stays folded",
  },
  {
    press: needsYou,
    say: "The question waits on you, with its reasons above it",
  },
]);

export { reasoningBeforeAQuestion as "reasoning-before-a-question" };
