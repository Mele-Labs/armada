// Bridge's calls for Triggers and for steps added to one Job, kept out of `commands.ts` for its
// length. `useCommands` hands the acts its own `setOutcome`, so a refusal goes where every
// command's goes, in Fleet's sentence.

import type { Outcome, HoldAct, TriggerFixChoice } from "@armada/protocol";

import type { ReadingTrigger, RemovingTrigger, SavingTrigger } from "../../shared/triggers";
import type { AddingStep, AddStepAnswer, RemovingStep, RemoveStepAnswer } from "../../shared/added-steps";

/** Triggers: what the picked repository runs, one as its file holds it, a save and a removal. */
export const readTriggers = () => window.armada.readTriggers();
export const readTrigger = (reading: ReadingTrigger) => window.armada.readTrigger(reading);
export const saveTrigger = (saving: SavingTrigger) => window.armada.saveTrigger(saving);
export const removeTrigger = (removing: RemovingTrigger) => window.armada.removeTrigger(removing);

export function triggerActs(setOutcome: (outcome: Outcome) => void) {
  /**
   * Where a failed Trigger's held fix goes. An accepted one says nothing, since the branch moves a
   * beat later as `job.trigger_changed` re-reads the Job. The answer tells the branch whether to
   * ask again.
   */
  async function chooseTriggerFix(jobId: string, by: { trigger: string } | { addition: string }, choice: TriggerFixChoice): Promise<{ ok: boolean }> {
    const answer = await window.armada.chooseTriggerFix(jobId, { ...by, choice });
    if (!answer.ok) setOutcome(answer);
    return { ok: answer.ok };
  }

  /** Run a held Trigger's Command again. */
  async function rerunTrigger(jobId: string, body: HoldAct): Promise<{ ok: boolean }> {
    const answer = await window.armada.rerunTrigger(jobId, body);
    if (!answer.ok) setOutcome(answer);
    return { ok: answer.ok };
  }

  /** Let a held Trigger go. */
  async function skipTrigger(jobId: string, body: HoldAct): Promise<{ ok: boolean }> {
    const answer = await window.armada.skipTrigger(jobId, body);
    if (!answer.ok) setOutcome(answer);
    return { ok: answer.ok };
  }

  /**
   * Add a step to a Job underway, or take one off before it fires. **Not through `act`**, `addTask`'s
   * reason: the answer is the row, which the panel that filled it in needs to draw it at once.
   */
  async function addJobStep(adding: AddingStep): Promise<AddStepAnswer> {
    const answer = await window.armada.addJobStep(adding);
    setOutcome(answer.ok ? { ok: true } : answer.outcome);
    return answer;
  }

  async function removeJobStep(removing: RemovingStep): Promise<RemoveStepAnswer> {
    const answer = await window.armada.removeJobStep(removing);
    setOutcome(answer.ok ? { ok: true } : answer.outcome);
    return answer;
  }

  return { chooseTriggerFix, rerunTrigger, skipTrigger, addJobStep, removeJobStep };
}
