// Answering a card in Helm's dock (#936). Each kind's answer calls the endpoint Job detail
// already sends to; this is the wiring, apart from `App.tsx` for the reason `commands.ts` is.

import { useCallback, useMemo, useRef, useState } from "react";
import type { CommandAnswer, HelmCallAnswer, JudgeAnswer, JobSummary, Outcome } from "@armada/protocol";
import type { DockActs, Outstanding } from "@armada/screens";
import { choiceNamed, dockCardsOf, outstandingId, refusalWords, ruleForKit } from "@armada/screens";

import { pointHelm, type useCommands } from "./commands";

/**
 * `onAnswer` sends by kind and keeps the last refusal a card's own answer drew, by the card's own
 * id — cleared on a fresh attempt. `answering` reads `commands.acting`, the one flag every other
 * answer on this window already sets, so a card and a Job detail band open on the same Job agree
 * about whether something is on its way to Fleet.
 *
 * **Memoised on `refusals` and `commands.acting` alone.** `commands` itself is rebuilt every
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
      answering: (question: Outstanding) =>
        question.kind === "helm"
          ? commands.answeringHelm === question.call.call
          : commands.acting === question.job_id,
      refusalFor: (question: Outstanding) => refusals[outstandingId(question)],
    }),
    [onAnswer, refusals, commands.acting, commands.answeringHelm],
  );
}

/**
 * The dock's cards. "Discuss with Helm" points it at the card's own repository, and the picker never
 * moves for it. `questions` is the dock's own block; `asks` is Helm's, drawn at the end of its thread
 * where the reply they stopped is — #1519.
 */
export function useDockCards(
  questions: Parameters<typeof dockCardsOf>[0],
  jobs: readonly JobSummary[],
  repositories: Parameters<typeof dockCardsOf>[2],
  commands: ReturnType<typeof useCommands>,
) {
  const answering = useDockAnswering(commands);
  const onDiscuss = useCallback((_question: Outstanding, job: JobSummary) => pointHelm(job.owner_manifest_id), []);
  return useMemo(
    () => dockCardsOf(questions, jobs, repositories, null, { ...answering, onDiscuss }),
    [questions, jobs, repositories, answering, onDiscuss],
  );
}

function sent(
  commands: ReturnType<typeof useCommands>,
  question: Outstanding,
  answer: string,
): Promise<Outcome> {
  switch (question.kind) {
    case "drone":
      return commands.answer(question.job_id, question.asking.question_id, answer);
    case "command": {
      const named = choiceNamed(answer);
      // An answer this build never drew is sent as it came, and Fleet refuses what it did not offer.
      if (named === undefined) return commands.answerCommand(question.job_id, question.waiting.call, answer as CommandAnswer);
      // Always allow on this machine has no picker in the dock, so it sends the rule Fleet pre-selected.
      return named.scope === undefined
        ? commands.answerCommand(question.job_id, question.waiting.call, named.answer)
        : commands.answerCommand(
            question.job_id,
            question.waiting.call,
            named.answer,
            undefined,
            ruleForKit(question.waiting),
            named.scope,
          );
    }
    case "judge":
      return commands.answerJudge(question.job_id, question.question.asked_at, answer as JudgeAnswer);
    case "helm":
      return commands.answerHelmCall(question.call.call, answer as HelmCallAnswer);
  }
}
