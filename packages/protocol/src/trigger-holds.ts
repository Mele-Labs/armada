// A Trigger that holds its Job, and what the owner does about it. `crates/ipc/src/trigger_holds.rs`.
// Since 23.68.

import type { TriggerFiringState, TriggerMoment } from "./triggers";

/** The most pressing of the three, in this order: a hold, a fix waiting on a choice, a failure nobody could repair. */
export type JobAlertKind = "held" | "fix_ready" | "failed";

/** What a Board row's bell is about. A row draws a mark and a tooltip from it and composes no sentence. */
export type JobAlert = {
  kind: JobAlertKind;
  /** The Trigger's name, or the added step's command, skill or brief. */
  trigger: string;
  when: TriggerMoment;
  /** The step it fired at. For `pr_opened`, the delivering one. */
  step: string;
};

/** `rerun_trigger`'s and `skip_trigger`'s body. Exactly one of the two is set. */
export type HoldAct = { trigger?: string; addition?: string };

/** What an act came to: where the firing stands, and whether the job holds nothing now. */
export type HoldSettled = { state: TriggerFiringState; released: boolean };
