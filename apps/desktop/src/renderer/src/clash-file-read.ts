// One file of a write clash, as the Sessions draft wants it: each side's patch, or the words for why there is none.

import type { ClashFileRead } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";

import type { ClashFileAnswer } from "../../shared/api/clashes";

/** What a Fleet without `GET /clashes/file` is said to be. Fleet never answers 404 itself: a clash that is gone is a 409. */
const NO_ROUTE = "This Fleet cannot show the files of a clash.";

export function clashFileRead(answer: ClashFileAnswer): ClashFileRead {
  if (answer.ok) return { ok: true, ours: answer.value.ours, theirs: answer.value.theirs };
  const { outcome } = answer;
  if (outcome.ok === false && outcome.why === "transport" && outcome.fault.why === "unanswerable" && outcome.fault.status === 404) return { ok: false, said: NO_ROUTE };
  return { ok: false, said: refusalWords(outcome) };
}
