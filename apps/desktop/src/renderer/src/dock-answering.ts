// Answering a card in Helm's dock (#936). This is the wiring, apart from `App.tsx` for the reason
// `commands.ts` is.

import { useCallback, useMemo, useRef, useState } from "react";
import type { HelmCallAnswer, Outcome } from "@armada/protocol";
import type { DockActs, Outstanding } from "@armada/screens";
import { dockQuestionsOf, outstandingId, refusalWords } from "@armada/screens";

import type { useCommands } from "./commands";

/**
 * `onAnswer` sends the answer and keeps the last refusal a card's own answer drew, by the card's own
 * id — cleared on a fresh attempt. `answering` reads `commands.answeringHelm`, so a card
 * is drawn off while its own answer is on its way to Fleet.
 *
 * **Memoised on `refusals` and `commands.answeringHelm` alone.** `commands` itself is rebuilt every
 * render — `commands.ts`'s own choice — so closing over it directly would defeat the memo `App.tsx`
 * builds the dock's cards under; a ref carries the latest one instead.
 */
export function useDockAnswering(commands: ReturnType<typeof useCommands>): DockActs {
  const [refusals, setRefusals] = useState<Record<string, string>>({});
  const latest = useRef(commands);
  latest.current = commands;

  const onAnswer = useCallback(async (question: Outstanding, answer: string): Promise<void> => {
    const id = outstandingId(question);
    setRefusals(({ [id]: _dropped, ...rest }) => rest);
    const outcome = await sent(latest.current, question, answer);
    if (!outcome.ok) setRefusals((was) => ({ ...was, [id]: refusalWords(outcome) }));
  }, []);

  return useMemo(
    () => ({
      onAnswer,
      answering: (question: Outstanding) => commands.answeringHelm === question.call.call,
      refusalFor: (question: Outstanding) => refusals[outstandingId(question)],
    }),
    [onAnswer, refusals, commands.answeringHelm],
  );
}

/**
 * The dock's cards. `asks` is Helm's, drawn at the end of its thread where the reply they stopped
 * is — #1519.
 */
export function useDockCards(
  questions: Parameters<typeof dockQuestionsOf>[0],
  repositories: Parameters<typeof dockQuestionsOf>[1],
  commands: ReturnType<typeof useCommands>,
) {
  const answering = useDockAnswering(commands);
  return useMemo(() => dockQuestionsOf(questions, repositories, null, answering), [questions, repositories, answering]);
}

function sent(
  commands: ReturnType<typeof useCommands>,
  question: Outstanding,
  answer: string,
): Promise<Outcome> {
  return commands.answerHelmCall(question.call.call, answer as HelmCallAnswer);
}
