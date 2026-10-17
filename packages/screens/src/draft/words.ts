// The verbs and tokens the draft's own enum values render as. Draft, for
// `crates/core-model/domain/enum-verbs.toml`.
//
// Source of truth today: `packages/components/src/generated/vocabulary.ts`,
// generated from that registry, which is where every word a surface renders
// comes from. Nothing here is a second copy of a word that file already has —
// each entry below is a value **no registry declares yet**.
//
// **A draft word goes through `enum-verbs.toml` at promotion** (#1532), and the
// entry here is deleted in the same change. Until then this is where a draft
// value gets a verb, so no board types one inline — which is the drift
// `lib/job-states.js` was deleted for.
//
// **Every token is one `packages/tokens/src/status.css` already defines.**
// None of the boards' off-token hexes reaches code (#1530).

import type { Rendering } from "@armada/components/src/generated/vocabulary";

import type { CaseRunOutcome, CaseState } from "./cases";

/**
 * A draft word, shaped exactly like a generated `Rendering` minus its glyph.
 *
 * **No icon.** `docs/contracts/iconography.md` defaults to none, and only
 * glyphs in `packages/icons/icons/` may be drawn — picking one for a value
 * that has no registry row would be minting vocabulary in the place this file
 * exists to stop.
 */
export type DraftWord = Omit<Rendering, "icon" | "hint">;

/**
 * **Task state was promoted at protocol 22.0** (spike 022, slice 1a), with
 * `handed_in` and `failed` on the wire. It is `TASK_STATE` in
 * `@armada/components`, generated from `verbs.task_state.*`, glyphs and all.
 */

/**
 * **Group state was promoted on 29 Sep 2026** and its map is gone from here.
 * It is `GROUP_STATE` in `@armada/components`, generated from
 * `verbs.group_state.*` in `crates/core-model/domain/enum-verbs.toml` — same
 * verbs and tokens, plus the glyph a draft word could never carry.
 *
 * **That glyph is why it moved.** A group's state had to draw as a tag beside
 * a plan's groups (the owner, 29 Sep), and this file withholds icons on
 * purpose: a value with no registry row has nothing to attach one to. So the
 * row was written rather than the rule bent. It has no registry *file* and no
 * enum, which `enum-verbs.toml`'s header records as the case `advance_gate`
 * and `gaming_pattern` already are — Fleet has no group yet, and `#1545` is
 * where the shape settles.
 */

/**
 * **`classifying` went at 22.0**: the status is `proposing`, already on the
 * wire with its own registry row (spike 022).
 */

/**
 * What became of a run — **never of the work**.
 *
 * `run_failed` is the harness falling over and says nothing about whether the
 * change is right. A case with no spec is `not_run`, which reads as not
 * covered.
 */
export const CASE_RUN_OUTCOME_WORDS: Readonly<Record<CaseRunOutcome, DraftWord>> = {
  ran: { verb: "ran", badgeStatus: "running", statusToken: "--status-running" },
  run_failed: {
    verb: "the run failed",
    badgeStatus: "completed-failed",
    statusToken: "--status-completed-failed",
  },
  not_run: {
    verb: "not covered",
    badgeStatus: "not-started",
    statusToken: "--status-not-started",
  },
};

/**
 * A criterion nothing recorded a verdict for.
 *
 * **Not "not covered", which is a case with no spec.** They are two different
 * facts: a case with no spec has nothing to run, and this is a criterion whose
 * answer was never written down — a Check-verified one, since nothing on the
 * wire links a Check to the criterion it answers. One sentence for two facts
 * teaches a reader to distrust both (owner, 22 Sep 2026).
 */
export const CRITERION_NO_VERDICT_WORD: DraftWord = {
  verb: "no verdict recorded",
  badgeStatus: "not-started",
  statusToken: "--status-not-started",
};

/** What a task owes. `dropped` is a case that fell out, never one that passed. */
export const CASE_STATE_WORDS: Readonly<Record<CaseState, DraftWord>> = {
  owed: { verb: "owed", badgeStatus: "awaiting-review", statusToken: "--status-awaiting-review" },
  dropped: { verb: "dropped", badgeStatus: "killed", statusToken: "--status-killed" },
  added: { verb: "added", badgeStatus: "not-started", statusToken: "--status-not-started" },
};

/**
 * Every draft word in one list, so `#1545` can read what this module owes the
 * registry without opening each map.
 */
export const DRAFT_VOCABULARIES: readonly {
  readonly vocabulary: string;
  readonly words: Readonly<Record<string, DraftWord>>;
}[] = [
  { vocabulary: "case_run_outcome", words: CASE_RUN_OUTCOME_WORDS },
  { vocabulary: "case_state", words: CASE_STATE_WORDS },
  { vocabulary: "criterion_reading", words: { no_verdict: CRITERION_NO_VERDICT_WORD } },
];
