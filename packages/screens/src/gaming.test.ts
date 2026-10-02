// The gaming check's reading: where a flag's lines are, what the rail and the
// panel say about it, and what Send it back tells a Drone. #1079, #1672.

import { describe, expect, it } from "vitest";
import type { Diff, Flagged } from "@armada/protocol";

import { freshStep } from "./fixtures/build/base";
import { flagSaid, flagsOf, hunkFor, sentBackWords } from "./gaming";

const JOB = "01M22TYSAE0023MADDP5ZQEYGW";
const FILE = "packages/settings/test/useColumnSelectors.test.ts";

/** Two hunks: an assertion replaced (line 40 of the post-image), and two removed. */
const PATCH = [
  `diff --git a/${FILE} b/${FILE}`,
  `--- a/${FILE}`,
  `+++ b/${FILE}`,
  '@@ -38,5 +38,5 @@ describe("useColumnSelectors", () => {',
  '   it("keeps hidden columns out of the visible set", () => {',
  "     const visible = selectVisible(state);",
  '-    expect(visible).toEqual(["name", "status", "owner"]);',
  "+    expect(visible.length).toBeGreaterThan(0);",
  "   });",
  "@@ -52,6 +52,4 @@",
  '   it("drops a column that was hidden", () => {',
  '-    expect(next.hidden).toContain("owner");',
  '-    expect(selectVisible(next)).not.toContain("owner");',
  "     expect(next.version).toBe(state.version + 1);",
  "   });",
].join("\n");

function diff(jobId = JOB): Diff {
  return { state: "read", jobId, work: { files: [], plan_declared: true, patch: PATCH } };
}

function flag(over: Partial<Flagged> = {}): Flagged {
  return { attempt: 1, pattern: "assertion_weakened", cited: "", at: { file: FILE }, ...over };
}

describe("where a flag's lines are", () => {
  it("finds a flag on an added line by that line, counted off the hunk header", () => {
    const found = hunkFor(flag({ at: { file: FILE, line: 40 } }), diff(), JOB);
    expect(found?.lines[0]?.text).toMatch(/^@@ -38,5 \+38,5 @@/);
    expect(found?.lines.some((line) => line.text.includes("toBeGreaterThan"))).toBe(true);
  });

  it("finds a flag on a removed line by the words it quotes, since it has no line", () => {
    const cited = '`expect(selectVisible(next)).not.toContain("owner")` was taken out, and nothing replaces it.';
    const found = hunkFor(flag({ cited }), diff(), JOB);
    expect(found?.lines[0]?.text).toBe("@@ -52,6 +52,4 @@");
  });

  it("draws nothing where no line carries the words, rather than a hunk near them", () => {
    expect(hunkFor(flag({ cited: "`assert_eq!(routes.len(), served.len())`" }), diff(), JOB)).toBeUndefined();
  });

  it("draws nothing for a line no hunk holds", () => {
    expect(hunkFor(flag({ at: { file: FILE, line: 120 } }), diff(), JOB)).toBeUndefined();
  });

  it("draws nothing from another Job's patch", () => {
    expect(hunkFor(flag({ at: { file: FILE, line: 40 } }), diff("another"), JOB)).toBeUndefined();
  });
});

describe("what the rail and the panel say", () => {
  const held = flag();
  const cleared: Flagged = { ...flag(), cleared: { why: "a doc comment, not a check" } };

  it("splits the flags that hold the step from those a second reading cleared", () => {
    const read = flagsOf(freshStep("implement", "Implement", 2), [held, cleared]);
    expect(read.held).toEqual([held]);
    expect(read.cleared).toEqual([cleared]);
  });

  it("says what happened in the registry's headline, and the wire spelling only without one", () => {
    expect(flagSaid(held)).toBe(
      "An assertion was removed or loosened, which was judged to weaken the test coverage",
    );
    expect(flagSaid(flag({ pattern: "a_pattern_fleet_learned" }))).toBe("a_pattern_fleet_learned");
  });
});

describe("the words Send it back redirects a Drone with", () => {
  const cited = flag({ cited: "`expect(a).toBe(b)` was removed", asked: "Is that assertion made nowhere else?" });

  it("carry what the pattern caught, what was cited, what was asked, and the note", () => {
    const words = sentBackWords([cited], "  Put it back  ");
    expect(words).toContain(
      "An assertion was removed or loosened, which was judged to weaken the test coverage.",
    );
    expect(words).toContain("It cited: `expect(a).toBe(b)` was removed");
    expect(words).toContain("It asked: Is that assertion made nowhere else?");
    expect(words).toContain("The person's note: Put it back");
  });

  it("say nothing about a note nobody wrote", () => {
    expect(sentBackWords([cited], undefined)).not.toContain("note");
  });
});
