// What a save of a Trigger answers, as the editor reads it: Fleet's own sentence on a refusal, and
// whether the file was already there. One reading, for the Workflow editor's save and the one that
// keeps a Job's added step for every Job.

import type { TriggerSavedAnswer } from "@armada/components";
import { said } from "@armada/screens/src/copy";

import type { TriggerSaveAnswer } from "../../shared/triggers";

export function savedAs(answer: TriggerSaveAnswer): TriggerSavedAnswer {
  if (answer.ok) return { ok: true, saved: answer.saved };
  const { outcome } = answer;
  if (outcome.ok === false && outcome.why === "refused") {
    return { ok: false, said: outcome.error.message, ...(outcome.error.code === "fleet.trigger_exists" ? { exists: true as const } : {}) };
  }
  return { ok: false, said: said(outcome) };
}
