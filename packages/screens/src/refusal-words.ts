// A refusal in words, so a Job's lessons can say one.

import type { Outcome } from "@armada/protocol";
import { said } from "./copy";

/** A card's own refusal, in words — Fleet's for a real refusal, Bridge's own sentence otherwise. */
export function refusalWords(outcome: Outcome): string {
  return outcome.ok ? "" : outcome.why === "refused" ? outcome.error.message : said(outcome);
}
