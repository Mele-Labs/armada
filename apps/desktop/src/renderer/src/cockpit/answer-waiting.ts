// The one function a waiting item is answered through. The wire's route is
// `window.armada.answerWaiting({ session_id, item_id, choice?, text?, mode? })`; until it is served
// this lands the answer in the Session through the path the Sessions page already uses (`draft.answer`),
// and keeps what it was asked to send, so a test or a walk can read the body. Mock only.

import type { SessionsDraft } from "@armada/screens/src/draft/sessions";

import { permissionOf, waitingOf } from "./waiting";

/** The body of `answerWaiting`. `mode` hands the decision to the agent: `best` weighs it, `quick` gets it done. */
export type AnswerWaiting = { session_id: string; item_id: string; choice?: string; text?: string; mode?: "best" | "quick" };

const sent: AnswerWaiting[] = [];

/** What was sent, oldest first. The mock's own record; a real Fleet has the route's. */
export const answersSent = (): readonly AnswerWaiting[] => sent;

export function answerWaiting(draft: SessionsDraft | undefined, request: AnswerWaiting): void {
  sent.push(request);
  const session = draft?.get().find((one) => one.id === request.session_id);
  const item = session === undefined ? undefined : waitingOf(session).find((one) => one.id === request.item_id);
  if (draft === undefined || item === undefined) return;
  // Handing the decision over answers with the first thing offered, which the mock cannot weigh.
  const chosen = request.choice ?? request.text ?? item.options?.[0]?.label;
  if (item.source === "permission") {
    draft.answer(request.session_id, permissionOf(chosen ?? "") ?? "refuse");
    return;
  }
  if (item.source === "ask_card") {
    draft.answer(request.session_id, undefined, [{ question: item.text, chosen: chosen === undefined ? [] : [chosen] }]);
    return;
  }
  draft.answer(request.session_id);
}
