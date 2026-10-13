// The answers a person gives a Job's waiting Drone, and the Check acts on a running gate.
//
// Beside `command.ts` rather than inside it, `review.ts`'s reason: the file had passed the length
// the gate refuses. Each takes `guarded`, which is `JobCommands.act` with the Job's in-flight set
// already chosen, so nothing here holds a connection or a set of its own.

import type {
  AlwaysAllowScope,
  AnswerCommand,
  AnswerMode,
  CheckNamed,
  ChosenAnswer,
  CommandAnswer,
  JudgeAnswer,
  JudgeAnswered,
  Outcome,
} from "@armada/protocol";
import { ask, route, type Answer } from "./request";

/** One request under a Job, sent under the caller's in-flight guard. */
export type Guarded = (jobId: string, send: (port: number) => Promise<Answer>) => Promise<Outcome>;

/**
 * Answer the question the job's drone asked. **The job comes back
 * unchanged** — it was `running` while it waited and is `running` now; what
 * moved is the drone, handed the answer as a turn.
 *
 * Nothing is validated here beyond emptiness. Which labels were offered is
 * fleet's, read off a working slot no window holds, so a label this window
 * believes in and fleet does not is a 409 rather than a guess.
 */
export async function answerQuestion(
  guarded: Guarded,
  jobId: string,
  questionId: string,
  chose: string | undefined,
  mode?: AnswerMode,
): Promise<Outcome> {
  // A mode alone is an answer; a label that is blank is not, with or without one.
  if (chose === undefined ? mode === undefined : chose.trim() === "") {
    return { ok: false, why: "empty_instruction" };
  }
  const body: ChosenAnswer = {
    question_id: questionId,
    ...(chose === undefined ? {} : { chose }),
    ...(mode === undefined ? {} : { mode }),
  };
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, "answer_question"), body));
}

/**
 * Allow or reject a command the job's drone was not given. **One act for
 * both places a person meets one**, because the call id already says which:
 * a command a drone is waiting on is answered in place, and a refused row on
 * a stopped job moves the job on.
 *
 * **Under `answering`, beside the question's answer.** Both hand a waiting
 * drone a person's choice, and a drone held inside one call is not asking
 * the other. Which answers were offered is Fleet's, so one this window
 * believes in and Fleet does not is a 409 rather than a guess.
 */
export async function answerCommand(
  guarded: Guarded,
  jobId: string,
  call: string,
  answer: CommandAnswer,
  note?: string,
  rule?: string,
  scope?: AlwaysAllowScope,
  mode?: AnswerMode,
): Promise<Outcome> {
  // Nothing rather than an empty string. A bare refusal is the body every
  // fleet before 11.5 took, and `{"note":""}` is a person's words nobody
  // wrote — `restart_step`'s rule, on the seam that established it.
  const said = note?.trim() ?? "";
  const body: AnswerCommand = {
    call,
    answer,
    ...(said === "" ? {} : { note: said }),
    // Only always_allow reads it, and only the caller's own picker sends
    // one — Fleet 409s a name outside that command's own candidates.
    ...(rule === undefined ? {} : { rule }),
    // Only always_allow reads it, and only the Kit choice sends one: absent
    // is the repository, which is what every Fleet before 23.35 kept it in.
    ...(scope === undefined ? {} : { scope }),
    ...(mode === undefined ? {} : { mode }),
  };
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, "answer_command"), body));
}

/**
 * Answer the question a judge refusal opened: agree with it, disagree for
 * this step, or disagree and stand the criterion down for the repository.
 * One press is the whole answer — `note` is never required.
 */
export function answerJudge(
  guarded: Guarded,
  jobId: string,
  askedAt: string,
  answer: JudgeAnswer,
  note?: string,
  mode?: AnswerMode,
): Promise<Outcome> {
  const body: JudgeAnswered = { answer, asked_at: askedAt, note, ...(mode === undefined ? {} : { mode }) };
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, "answer_judge"), body));
}

/**
 * Run one Check of the running gate again, or end it recorded as skipped. **Under `rerunningChecks`,
 * the set `rerunChecks` uses**: that one answers a stopped step and these answer a running gate, so a
 * Job never has both out and they need no set apart. The route answers when the Check is restarted
 * rather than when it ends, so the default wait holds.
 */
export function actOnCheck(guarded: Guarded, jobId: string, act: "retry_check" | "skip_check", check: string): Promise<Outcome> {
  const body: CheckNamed = { check };
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, act), body));
}

/** End every Check of the running gate that has not finished. No body. */
export function skipChecks(guarded: Guarded, jobId: string): Promise<Outcome> {
  return guarded(jobId, (port) => ask(port, "POST", route(jobId, "skip_checks")));
}
