// Best or quick on a Job's ask, sent to Fleet. A Drone's question takes the mode alone. A Judge's is
// answered as agreed, so the step goes back to a Drone with the finding, and the mode says how it goes
// about it. A command takes none, since it is answered by one of its offers; nor does an ask the mock
// drew, which names no Fleet question. Those give `undefined`.

import type { AskedOf } from "@armada/screens/src/draft/now";
import type { AnswerMode } from "@armada/protocol";
import { refusalWords } from "@armada/screens/src/refusal-words";

/** What sending it came to: Fleet took it, or refused in these words. */
export type JobAnswered = { kind: "answered" } | { kind: "refused"; said: string };

export function answerJobAsk(jobId: string, asked: AskedOf | undefined, mode: AnswerMode): (() => Promise<JobAnswered>) | undefined {
  if (asked === undefined || asked.kind === "command") return undefined;
  return async () => {
    const done =
      asked.kind === "judge"
        ? await window.armada.answerJudge(jobId, asked.at, "agree", undefined, mode)
        : await window.armada.answerQuestion(jobId, asked.id, undefined, mode);
    return done.ok ? { kind: "answered" } : { kind: "refused", said: refusalWords(done) };
  };
}
