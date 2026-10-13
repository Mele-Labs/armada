// What the owner does to a Job's Commander: write to it, and pick on the card it put out. Apart
// from `App.tsx` for the reason `commands.ts` is. Neither goes through `commands.acting`: the words
// queue for the next turn and never interrupt one, and a pick is one of a closed set Fleet offered.

import { refusalWords } from "@armada/screens/src/refusal-words";

import type { JobAnswered } from "./cockpit/answer-job";

/**
 * `onAnswerCard` is the Cockpit's: it hands back whether Fleet took the pick, so the call clears
 * only then. `job` is what a Job's own page takes, which says a refusal in the toast itself.
 */
export function useCommanderActs(tell: (sentence: string) => void) {
  const onAnswerCard = async (jobId: string, card: string, option: string): Promise<JobAnswered> => {
    const done = await window.armada.answerCommanderCard(jobId, card, option);
    return done.ok ? { kind: "answered" } : { kind: "refused", said: refusalWords(done) };
  };
  const job = {
    onAskCommander: async (jobId: string, text: string): Promise<boolean> => {
      const done = await window.armada.askCommander(jobId, text);
      if (!done.ok) tell(refusalWords(done));
      return done.ok;
    },
    onAnswerCommander: async (jobId: string, card: string, option: string): Promise<boolean> => {
      const told = await onAnswerCard(jobId, card, option);
      if (told.kind === "refused") tell(told.said);
      return told.kind === "answered";
    },
  };
  return { onAnswerCard, job };
}
