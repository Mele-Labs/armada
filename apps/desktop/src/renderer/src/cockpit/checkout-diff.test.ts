// An uncommitted file's change on the checkout card: what each answer of Fleet draws as.

import { describe, expect, it } from "vitest";

import { checkoutDiffOf } from "./checkout-diff";

const DIFF = ["diff --git a/armada.yml b/armada.yml", "index 3f2a9c1..b84e0d7 100644", "--- a/armada.yml", "+++ b/armada.yml", "@@ -4,2 +4,3 @@", " merge:", "+  auto_merge: checks-pass", "   method: squash", ""].join("\n");

describe("the change of one uncommitted file", () => {
  it("is empty and says nothing while the read is in flight", () => {
    expect(checkoutDiffOf(undefined)).toEqual({ files: [] });
  });

  it("is the file's lines as git wrote them, with the marker kept", () => {
    const { files, emptyNote } = checkoutDiffOf({ ok: true, diff: DIFF });
    expect(emptyNote).toBeUndefined();
    expect(files.map((file) => file.path)).toEqual(["armada.yml"]);
    expect(files[0]?.lines.map((line) => [line.kind, line.text])).toEqual([
      ["hunk", "@@ -4,2 +4,3 @@"],
      ["context", " merge:"],
      ["added", "+  auto_merge: checks-pass"],
      ["context", "   method: squash"],
    ]);
  });

  it("says there are no lines changed where git printed nothing", () => {
    expect(checkoutDiffOf({ ok: true, diff: "" })).toEqual({ files: [], emptyNote: "No lines changed in this file." });
  });

  it("is Fleet's own sentence where it refused", () => {
    expect(checkoutDiffOf({ ok: false, said: "That file is not uncommitted in the checkout." })).toEqual({ files: [], emptyNote: "That file is not uncommitted in the checkout." });
  });

  it("is cut at the bound and says how much was drawn", () => {
    const long = `${DIFF}${Array.from({ length: 2500 }, (_, i) => `+line ${i}\n`).join("")}`;
    const { files, cut } = checkoutDiffOf({ ok: true, diff: long });
    expect(files[0]?.lines.length).toBeLessThanOrEqual(2000);
    expect(cut).toMatch(/^The first 2,000 of [\d,]+ lines\.$/);
  });
});
