// What Bridge asks the host for about Triggers, and what it is answered with.
// `main/triggers.ts` builds these from Fleet's four routes; nothing else does.

import type {
  AlertList,
  Outcome,
  RemoveTrigger,
  SaveTrigger,
  TriggerDefinition,
  TriggerLevel,
  TriggerList,
  TriggerMoment,
  TriggerRemoved,
  TriggerSaved,
} from "@armada/protocol";

/** `GET /alerts`: what is waiting on a person, for the picked repository or every one on All. */
export type AlertsRead = ({ ok: true } & AlertList) | { ok: false; outcome: Outcome };

/** `GET /triggers`, for the picked repository: what runs, what each replaced, and what was left out. */
export type TriggersRead = ({ ok: true } & TriggerList) | { ok: false; outcome: Outcome };

/** One Trigger by its identity, `when`, `step` and `name`. */
export type TriggerIdentity = { when: TriggerMoment; step?: string; name: string };

/** `GET /triggers/definition`: one file's text. `level` absent is the copy that runs. */
export type TriggerDefinitionRead = { ok: true; definition: TriggerDefinition } | { ok: false; outcome: Outcome };

/** `POST /triggers/save`. A refusal carries Fleet's own sentence on `outcome`. */
export type TriggerSaveAnswer = { ok: true; saved: TriggerSaved } | { ok: false; outcome: Outcome };

/** `POST /triggers/remove`. */
export type TriggerRemoveAnswer = { ok: true; removed: TriggerRemoved } | { ok: false; outcome: Outcome };

/** One save, and the Manifest it is checked against, which the picked repository's stands in for. */
export type SavingTrigger = { manifestId: string | null; body: SaveTrigger };

/** One removal, named as a save is. */
export type RemovingTrigger = { manifestId: string | null; body: RemoveTrigger };

export type ReadingTrigger = { identity: TriggerIdentity; level?: TriggerLevel };
