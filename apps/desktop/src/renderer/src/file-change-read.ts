// One file a Job wrote, as its Field's lens wants it: the change and its stamp, or a refusal with no words, since the lens says nothing
// of one and draws nothing.

import type { FileChangeRead } from "@armada/screens/src/draft/sessions";

import type { SessionFileChangeAnswer } from "../../shared/api/clashes";

export function fileChangeRead(answer: SessionFileChangeAnswer): FileChangeRead {
  if (answer.ok) {
    const { stamp, diff, cut, unchanged } = answer.value;
    return { ok: true, stamp, diff, ...(cut === true ? { cut: true as const } : {}), ...(unchanged === true ? { unchanged: true as const } : {}) };
  }
  const { outcome } = answer;
  // A Fleet without the route will never answer it: the lens stops asking.
  const noRoute = outcome.ok === false && outcome.why === "transport" && outcome.fault.why === "unanswerable" && outcome.fault.status === 404;
  return noRoute ? { ok: false, noRoute: true } : { ok: false };
}
