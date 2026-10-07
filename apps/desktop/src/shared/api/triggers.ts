// Triggers.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { ChooseTriggerFix, Outcome } from "@armada/protocol";

import type {
  ReadingTrigger,
  RemovingTrigger,
  SavingTrigger,
  TriggerDefinitionRead,
  TriggerRemoveAnswer,
  TriggerSaveAnswer,
  TriggersRead,
} from "../triggers";

export type TriggersApi = {
  /**
   * What the picked repository runs at each moment of a Job, with what each replaced and what was
   * left out; one Trigger as its file holds it; and a save or a removal, which Fleet checks with the
   * loader's own rules and refuses in its own words. A repository's own Trigger runs once it is on
   * `main`, and the answer says so.
   */
  readTriggers: () => Promise<TriggersRead>;
  readTrigger: (reading: ReadingTrigger) => Promise<TriggerDefinitionRead>;
  saveTrigger: (saving: SavingTrigger) => Promise<TriggerSaveAnswer>;
  removeTrigger: (removing: RemovingTrigger) => Promise<TriggerRemoveAnswer>;
  /**
   * Where a failed Trigger's held fix goes, onto the Job's branch or into a pull request of its own.
   * Fleet never chooses, so this is the owner's. A refusal comes back as the outcome, in Fleet's
   * own words, and leaves the firing as it was.
   */
  chooseTriggerFix: (jobId: string, body: ChooseTriggerFix) => Promise<Outcome>;
};

export type TriggersState = Record<never, never>;

export const TRIGGERS_NOTHING_YET: TriggersState = {};

export const TRIGGERS_CHANNELS = {
  readTriggers: "bridge:read-triggers",
  readTrigger: "bridge:read-trigger",
  saveTrigger: "bridge:save-trigger",
  removeTrigger: "bridge:remove-trigger",
  chooseTriggerFix: "bridge:choose-trigger-fix",
} as const;
