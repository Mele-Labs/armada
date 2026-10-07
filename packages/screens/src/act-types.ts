// The names of the acts one Job's header offers. A file of their own so `copy.ts`,
// which words them, and `Acts.tsx`, which arranges them, need not import each other.

/**
 * What the two kills, the redispatch, the two step-resuming acts and the
 * three answers to a step that stopped are called.
 *
 * **`rerun_gate` is the seventh and it is not a widening of the sixth.** An
 * override answers a gate that ruled; this answers a gate that ruled on
 * nothing. `crates/fleet/src/regating.rs` admits exactly the trigger
 * `overrulable()` refuses, so the two names partition rather than overlap.
 *
 * **`rerun_checks` is the eighth, beside the seventh and not a widening of
 * it.** It answers a step that stopped on a failed mechanical Check rather
 * than an undecided gate — `#1105`, `recovery.ts`'s third trigger.
 */
export type JobAct =
  | "kill_drone"
  | "kill_job"
  | "redispatch"
  | "redirect"
  | "restart_step"
  | "override_verdict"
  | "rerun_gate"
  | "rerun_checks"
  // The ninth, and the only one that acts on disk rather than on the record
  // or the machine. `armada clean` could already do it and refuses while Fleet
  // is running, which is exactly when a person wants the space back.
  | "reclaim_worktree"
  // The tenth, and the one act on this header that cannot be undone. It takes
  // the row `reclaim_worktree` leaves — `crates/ipc/operations/` argues
  // the split on the same row that argues this one's — so a person wanting
  // both sends both, and this is never the act a stray `Enter` reaches.
  | "forget_job"
  // The eleventh and twelfth, and the only two that keep the Job: a pause saves
  // its work on its branch and gives its slot back, and Resume takes one again.
  // Neither ends anything, so neither is plain red.
  | "pause_job"
  | "resume_job";

/**
 * The acts that confirm through the shared dialog. **Redirect and the override
 * are not two of them** — each carries a required field in its own dialog, so
 * each is its own confirmation and neither also routes through this one.
 *
 * **Nor are the two re-runs, and for the opposite reason.** Neither has a
 * dialog at all: nothing is destroyed, nothing is overruled and nothing is
 * committed, so there is no responsibility for a person to take on the record.
 * A confirmation here would say a re-run costs something, which would be the
 * screen inventing a cost Fleet does not charge.
 */
export type ConfirmableAct = Exclude<
  JobAct,
  "redirect" | "override_verdict" | "rerun_gate" | "rerun_checks"
>;

/**
 * The acts that confirm by being held where the header draws them as its face —
 * a control of their own, or a split button's face. Everywhere else, the menu
 * entries, `x` and the palette included, they still ask.
 */
export type HeldAct = Extract<ConfirmableAct, "kill_drone" | "kill_job">;
