// What a tag is on the wire: its kind and the forge's address go with an issue or a pull request, and a Job's
// own facts with a Job.

import { describe, expect, it } from "vitest";

import { wireTagOf } from "./session-uploads";

describe("a tag as the wire has it", () => {
  it("sends an issue with its kind, its number, its title and the forge's address", () => {
    expect(wireTagOf({ kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" })).toEqual({
      kind: "issue",
      id: "2214",
      title: "#2214 Sessions list drops a quiet Session",
      url: "https://forge.invalid/issues/2214",
    });
  });

  it("sends a pull request the same way, and leaves the address out of a tag that has none", () => {
    expect(wireTagOf({ kind: "pull_request", id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" })).toEqual({
      kind: "pull_request",
      id: "2219",
      title: "#2219 Pin the Bridge store clock",
      url: "https://forge.invalid/pull/2219",
    });
    const branch = wireTagOf({ kind: "branch", id: "fix/flaky-store", title: "fix/flaky-store" });
    expect(branch).toEqual({ kind: "branch", id: "fix/flaky-store", title: "fix/flaky-store" });
    expect("url" in branch).toBe(false);
  });

  it("sends a Job with the facts the ledger holds it by, once its slot is known", () => {
    const job = { number: 55, branch: "55-cap-the-retry-backoff", state: "running" as const };
    expect(wireTagOf({ kind: "job", id: "01J55", title: "55 Cap the retry backoff", job: { ...job, slot: 3 } })).toEqual({
      kind: "job",
      id: "01J55",
      title: "55 Cap the retry backoff",
      job: { ...job, slot: 3 },
    });
    expect("job" in wireTagOf({ kind: "job", id: "01J55", title: "55 Cap the retry backoff", job })).toBe(false);
  });
});
