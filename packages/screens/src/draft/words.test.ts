// Every draft word is a real token, and no draft word is a second copy.

import {
  ADVANCE_GATE,
  GROUP_STATE,
  JOB_STATUS,
  STEP_STATE,
} from "@armada/components/src/generated/vocabulary";
import { describe, expect, it } from "vitest";

import {
  CASE_RUN_OUTCOME_WORDS,
  CLASSIFYING_WORD,
  CRITERION_NO_VERDICT_WORD,
  DRAFT_VOCABULARIES,
  TASK_STATE_WORDS,
} from "./words";

/** The tokens `packages/tokens/src/status.css` defines, by the stem it names. */
const STATUS_TOKENS = [
  "not-started",
  "running",
  "awaiting-review",
  "awaiting-approval",
  "escalated",
  "completed-success",
  "completed-failed",
  "rejected",
  "killed",
  "piloted",
  "awaiting-attestation",
];

describe("every draft word renders from the token scale", () => {
  it("names a status token that exists, and never a hex", () => {
    for (const { vocabulary, words } of DRAFT_VOCABULARIES) {
      for (const [value, word] of Object.entries(words)) {
        expect(
          STATUS_TOKENS.includes(word.badgeStatus ?? ""),
          `${vocabulary}.${value} names ${word.badgeStatus}`,
        ).toBe(true);
        expect(word.statusToken).toBe(`--status-${word.badgeStatus}`);
      }
    }
  });

  it("gives every value a verb, so nothing renders as a wire spelling", () => {
    for (const { words } of DRAFT_VOCABULARIES) {
      for (const word of Object.values(words)) {
        expect(word.verb?.length ?? 0).toBeGreaterThan(0);
      }
    }
  });

  it("carries no glyph, because none of these is in the icon registry", () => {
    for (const { words } of DRAFT_VOCABULARIES) {
      for (const word of Object.values(words)) {
        expect("icon" in word).toBe(false);
      }
    }
  });
});

describe("what the registry already answers is not restated here", () => {
  it("leaves the gate words to the generated vocabulary", () => {
    expect(ADVANCE_GATE["manifest_rule:auto_merge"]?.verb).toBeTruthy();
    for (const { vocabulary } of DRAFT_VOCABULARIES) {
      expect(vocabulary).not.toBe("advance_gate");
    }
  });

  it("spells classifying, which the registry knows only as another word", () => {
    expect(JOB_STATUS["classifying"]).toBeUndefined();
    expect(CLASSIFYING_WORD.verb).toBe("classifying");
  });
});

describe("the values that have no registry row at all", () => {
  // **Group state is no longer one of them.** It was promoted on 29 Sep 2026
  // so it could carry a glyph, which a draft word cannot: `GROUP_STATE` in the
  // generated vocabulary now holds all eight, and `enum-verbs.toml` holds the
  // rows. This claim replaces one that counted six group states no registry
  // spelled — true until the rows were written, and the reason they were.
  it("has promoted group state out, glyphs and all", () => {
    expect(DRAFT_VOCABULARIES.map((one) => one.vocabulary)).not.toContain("group_state");

    const states = ["pending", "running", "joining", "checking", "passed", "failed", "retrying", "landed"];
    for (const state of states) {
      expect(GROUP_STATE[state], state).toBeDefined();
      expect(GROUP_STATE[state]?.icon, state).not.toBeNull();
    }
  });

  // The glyphs are the step's one level down, which is what
  // `[conventions.step_activity_borrowing]` sanctions — never a mark minted
  // for a group alone.
  it("borrows every group glyph from a step or a Job, never a new one", () => {
    const borrowed = new Set(
      ["not_started", "running", "advanced", "retrying"].flatMap((state) => {
        const icon = STEP_STATE[state]?.icon;
        return icon === undefined || icon === null ? [] : [icon];
      }),
    );
    // `x` is the roster's and is `job_status.completed_failed`'s one level up.
    const failed = JOB_STATUS["completed_failed"]?.icon;
    if (failed !== undefined && failed !== null) borrowed.add(failed);

    for (const state of ["pending", "running", "passed", "failed", "retrying", "landed"]) {
      expect(borrowed, state).toContain(GROUP_STATE[state]?.icon);
    }
  });

  it("gives failed a word, which is the one task state the wire cannot send", () => {
    expect(TASK_STATE_WORDS.failed.verb).toBe("failed");
  });

  it("calls a case that did not run not covered, never passing", () => {
    expect(CASE_RUN_OUTCOME_WORDS.not_run.verb).toBe("not covered");
    expect(CASE_RUN_OUTCOME_WORDS.not_run.badgeStatus).not.toBe("completed-success");
  });

  // Two facts, two sentences: a case with no spec has nothing to run, and a
  // criterion with no verdict was never answered in the record.
  it("keeps an unanswered criterion apart from a case with no spec", () => {
    expect(CRITERION_NO_VERDICT_WORD.verb).toBe("no verdict recorded");
    expect(CRITERION_NO_VERDICT_WORD.verb).not.toBe(CASE_RUN_OUTCOME_WORDS.not_run.verb);
    expect(CRITERION_NO_VERDICT_WORD.badgeStatus).not.toBe("completed-success");
  });
});

describe("the list #1545 reads", () => {
  it("names every map in this module, so none is promoted by being forgotten", () => {
    expect(DRAFT_VOCABULARIES.map((entry) => entry.vocabulary)).toEqual([
      "task_state",
      // `group_state` was here until 29 Sep 2026, when it was promoted so it
      // could carry a glyph. Its absence is what this claim is for.
      "case_run_outcome",
      "case_state",
      "job_status",
      "criterion_reading",
    ]);
  });
});
