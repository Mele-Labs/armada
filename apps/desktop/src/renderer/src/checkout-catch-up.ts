// "Your checkout" run against a real Fleet: the card's one safe command, pressed. **The command string is not sent.** Fleet is
// asked to catch the repository up and runs the pull it knows, after reading the hold again, so what the card showed is only
// what the person pressed. What it answers plays as a run does: the words as output, then the exit.

import type { CheckoutCaughtUp } from "@armada/protocol";
import type { RunEvent } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { SessionActed } from "../../shared/api/sessions";

type On = (event: RunEvent) => void;

/** git's lines as a terminal draws them: every line ends in a carriage return and a feed, and the last does too. */
const lines = (text: string): string => (text === "" ? "" : text.replace(/\r?\n/g, "\r\n").replace(/(\r\n)?$/, "\r\n"));

/**
 * Asks Fleet to pull the checkout of the repository with this Manifest level with its base, and plays the answer. A refusal ends
 * the run with code 1 and Fleet's words. A pull has no way to be stopped once Fleet has it, so there is nothing for a stop to do.
 */
export function catchingUp(
  catchUp: (manifestId: string) => Promise<SessionActed<CheckoutCaughtUp>>,
  manifestId: string | undefined,
  on: On,
): { stop: () => void } {
  const stop = { stop: () => undefined };
  if (manifestId === undefined) {
    on({ kind: "out", text: "Fleet serves no Manifest for that repository.\r\n" });
    on({ kind: "end", code: 1 });
    return stop;
  }
  void catchUp(manifestId).then((done) => {
    if (!done.ok) {
      on({ kind: "out", text: `${refusalWords(done.outcome)}\r\n` });
      on({ kind: "end", code: 1 });
      return;
    }
    const out = lines(done.value.output);
    if (out !== "") on({ kind: "out", text: out });
    on({ kind: "end", code: done.value.code });
  });
  return stop;
}
