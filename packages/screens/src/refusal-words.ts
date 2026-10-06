// A refusal in words. Kept apart from `dock-questions.tsx`, the Helm dock's, so
// a Job's lessons can say one without importing it.

import type { Outcome } from "@armada/protocol";
import { said } from "./copy";

/** A card's own refusal, in words — Fleet's for a real refusal, Bridge's own sentence otherwise. */
export function refusalWords(outcome: Outcome): string {
  return outcome.ok ? "" : outcome.why === "refused" ? outcome.error.message : said(outcome);
}
