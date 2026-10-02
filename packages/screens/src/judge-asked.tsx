// A Judge's refusal a person is being asked about, with its three answers —
// the one block every surface draws for it.
//
// **One refusal, one answer, on every surface** (owner, 1 Oct 2026, `#1748`
// row 13). On his Job 2 a Judge refused the plan, and Overview asked the
// question while Plan offered Approve the plan, which skipped it, and the
// Workflow step panel offered nothing but Hold to stop. Overview's lead, Plan's
// lead and the step panel now draw this, with the same handlers, so the three
// cannot offer different acts at a refusal.

import { JudgeQuestion } from "@armada/components";
import type { JobDetail as JobWhole, JudgeAnswer, StepDetail } from "@armada/protocol";

import type { ActingAct } from "./pending";

export type JudgeAskedProps = {
  jobId: string;
  whole: JobWhole | null;
  /** The step being read. The block is drawn only where the question is on it. */
  step: StepDetail | undefined;
  /** Every control is refused while what is shown is not live. */
  stale: boolean;
  acting: boolean;
  actingAct?: ActingAct | undefined;
  /** Answer the question a judge refusal opened. `answer_judge` is one press. */
  onAnswerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
};

/** Whether a Judge's question is open on this step. */
export function judgeAskedOn(whole: JobWhole | null, step: StepDetail | undefined): boolean {
  return step !== undefined && whole?.judge_question?.step_id === step.step_id;
}

/**
 * The refusal, its finding, the note and the three answers. Nothing where no
 * question is open on the step.
 *
 * **`acting`, not `deciding`.** `onAnswerJudge` sends under `acting` —
 * `pending.ts`'s `answer_judge` is an `ActingAct`, never a `DecidingAct` — so
 * gating this on `deciding` left its own buttons live for the whole of the
 * press they had just sent. #1117.
 */
export function JudgeAsked({ jobId, whole, step, stale, acting, actingAct, onAnswerJudge }: JudgeAskedProps) {
  const question = whole?.judge_question;
  if (question === undefined || !judgeAskedOn(whole, step)) return null;
  return (
    <JudgeQuestion
      question={question.question}
      expected={question.expected}
      produced={question.produced}
      consequence={question.consequence}
      disabled={stale || acting}
      disabledNote={
        stale
          ? "This job is not live, so nothing can be sent."
          : acting
            ? "Something sent to this job is still on its way to Fleet."
            : undefined
      }
      pending={acting && actingAct === "answer_judge"}
      onAnswer={(answer, note) => onAnswerJudge(jobId, question.asked_at, answer, note)}
    />
  );
}
