// The gaming check's reading: what the rail and the panel say about it. #1079.

import { describe, expect, it } from "vitest";
import type { Flagged } from "@armada/protocol";

import { freshStep } from "./fixtures/build/base";
import { flagsOf } from "./gaming";

const FILE = "packages/settings/test/useColumnSelectors.test.ts";

function flag(over: Partial<Flagged> = {}): Flagged {
  return { attempt: 1, pattern: "assertion_weakened", cited: "", at: { file: FILE }, ...over };
}

describe("what the rail and the panel say", () => {
  const held = flag();
  const cleared: Flagged = { ...flag(), cleared: { why: "a doc comment, not a check" } };

  it("splits the flags that hold the step from those a second reading cleared", () => {
    const read = flagsOf(freshStep("implement", "Implement", 2), [held, cleared]);
    expect(read.held).toEqual([held]);
    expect(read.cleared).toEqual([cleared]);
  });
});
