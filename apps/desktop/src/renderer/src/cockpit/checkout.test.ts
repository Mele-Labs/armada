// A held checkout as a call: the one command that is safe to press, the files that clash first, and the reason a card leads with.

import { describe, expect, it } from "vitest";
import type { HeldCheckout } from "@armada/protocol";

import { checkoutCall } from "./checkout";

const ROOT = "/Users/user/armada";
const SAFE = `git -C ${ROOT} pull --ff-only --autostash`;

/** A checkout held by one uncommitted file, which is the shape the pull is safe for. */
const held = (over: Partial<HeldCheckout> = {}): HeldCheckout => ({ base: "main", uncommitted: ["armada.yml"], behind: 3, ahead: 0, ...over });

describe("the one command a held checkout offers", () => {
  it("is the autostash pull when the checkout is on the base, has uncommitted files, none clash and it is ahead of nothing", () => {
    expect(checkoutCall(ROOT, held()).command).toBe(SAFE);
  });

  it("quotes a root that is not a plain word", () => {
    expect(checkoutCall("/Users/user/my repo", held()).command).toBe("git -C '/Users/user/my repo' pull --ff-only --autostash");
    expect(checkoutCall("/Users/user/it's", held()).command).toBe(`git -C '/Users/user/it'\\''s' pull --ff-only --autostash`);
  });

  it("is absent when a file main also changed is uncommitted, because the stash could stop on a conflict", () => {
    expect(checkoutCall(ROOT, held({ clashing: ["armada.yml"] })).command).toBeUndefined();
  });

  it("is absent on another branch, whatever else is true", () => {
    expect(checkoutCall(ROOT, held({ on: "my-own-branch" })).command).toBeUndefined();
    expect(checkoutCall(ROOT, held({ on: "HEAD" })).command).toBeUndefined();
  });

  it("is absent when the checkout has commits the base lacks", () => {
    expect(checkoutCall(ROOT, held({ ahead: 2 })).command).toBeUndefined();
  });

  it("is absent when nothing is uncommitted, since Fleet catches a clean checkout up itself", () => {
    expect(checkoutCall(ROOT, held({ uncommitted: [] })).command).toBeUndefined();
    const { uncommitted: _none, ...without } = held();
    expect(checkoutCall(ROOT, without).command).toBeUndefined();
  });
});

describe("the files listed under Uncommitted", () => {
  const rows = (call: ReturnType<typeof checkoutCall>) => call.facts.find((one) => one.label === "Uncommitted")?.lines ?? [];

  it("put the files main also changed first, each group in path order", () => {
    const call = checkoutCall(ROOT, held({ uncommitted: ["z.rs", "b.rs", "a.rs", "y.rs"], clashing: ["y.rs", "b.rs"] }));
    expect(rows(call).map((line) => line.text)).toEqual(["b.rs", "y.rs", "a.rs", "z.rs"]);
  });

  it("carry a note on a clashing file and on no other", () => {
    const call = checkoutCall(ROOT, held({ uncommitted: ["a.rs", "b.rs"], clashing: ["b.rs"] }));
    expect(rows(call)).toEqual([{ text: "b.rs", note: "Also changed on main" }, { text: "a.rs" }]);
  });

  it("are left out of the facts when there are none", () => {
    expect(checkoutCall(ROOT, held({ uncommitted: [] })).facts.map((one) => one.label)).toEqual([]);
  });
});

describe("the reason a card leads with", () => {
  it("is a clash before anything else", () => {
    const call = checkoutCall(ROOT, held({ clashing: ["armada.yml"], ahead: 4, on: "other" }));
    expect(call.fact).toBe("Uncommitted changes main also changed");
  });

  it("is uncommitted changes before commits the base lacks and before the branch", () => {
    expect(checkoutCall(ROOT, held({ ahead: 4, on: "other" })).fact).toBe("Uncommitted changes");
  });

  it("is the commits the base lacks before the branch, once nothing is uncommitted", () => {
    expect(checkoutCall(ROOT, held({ uncommitted: [], ahead: 1, on: "other" })).fact).toBe("1 commit not on main");
    expect(checkoutCall(ROOT, held({ uncommitted: [], ahead: 2 })).fact).toBe("2 commits not on main");
  });

  it("is the branch when that is all there is", () => {
    expect(checkoutCall(ROOT, held({ uncommitted: [], on: "other" })).fact).toBe("On other");
  });

  it("is empty when nothing applies", () => {
    expect(checkoutCall(ROOT, held({ uncommitted: [] })).fact).toBe("");
  });

  it("counts the commits behind in the title", () => {
    expect(checkoutCall(ROOT, held({ behind: 1 })).title).toBe("1 commit behind main");
    expect(checkoutCall(ROOT, held({ behind: 880 })).title).toBe("880 commits behind main");
  });
});

describe("sending the uncommitted files as a pull request", () => {
  it("is offered under the same gate as the pull: on the base, ahead of nothing, files uncommitted, none clashing", () => {
    expect(checkoutCall(ROOT, held()).sendable).toBe(true);
    expect(checkoutCall(ROOT, held({ ahead: 2 })).sendable).toBeUndefined();
    expect(checkoutCall(ROOT, held({ clashing: ["armada.yml"] })).sendable).toBeUndefined();
    expect(checkoutCall(ROOT, held({ on: "my-own-branch" })).sendable).toBeUndefined();
    expect(checkoutCall(ROOT, held({ uncommitted: [] })).sendable).toBeUndefined();
  });

  it("is drawn from the hold Fleet published: the pull request replaces the command and the press", () => {
    const call = checkoutCall(ROOT, held({ pull_request: { number: 2402, auto_merge: true } }));
    expect(call.pullRequest).toEqual({ number: 2402, autoMerge: true });
    expect(call.command).toBeUndefined();
    expect(call.sendable).toBeUndefined();
    expect(call.facts.length).toBeGreaterThan(0);
  });

  it("keeps a pull request the forge would not auto-merge as open and without the mark", () => {
    expect(checkoutCall(ROOT, held({ pull_request: { number: 7, auto_merge: false } })).pullRequest).toEqual({ number: 7, autoMerge: false });
  });
});
