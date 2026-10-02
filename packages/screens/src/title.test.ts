import { describe, expect, it } from "vitest";

import { titleOf } from "./title";

describe("titleOf", () => {
  it("draws a request typed in markdown as one line of its words", () => {
    const title = "Say which was given back:\n\n- the **worktree** alone\n- the `branch` as well";
    expect(titleOf({ title })).toBe("Say which was given back: the worktree alone the branch as well");
  });

  it("leaves a plain title as it was, an underscore inside a name included", () => {
    expect(titleOf({ title: "Split settings_store's reducer" })).toBe("Split settings_store's reducer");
  });
});
