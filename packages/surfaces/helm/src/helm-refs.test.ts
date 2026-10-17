// What Helm's thread links, resolved against the shapes a real Fleet publishes: a Session's id and address as `sessions-wire` derives them,
// issues and pull requests as `#` holds them, and a repository's merge line as `list_merge_lines` answers it.

import { describe, expect, it } from "vitest";

import type { MergeLine } from "@armada/protocol";
import { branchOfPullRequest, scanRefs } from "./helm-refs";
import type { RefWorld } from "./helm-refs";

const WORLD: RefWorld = {
  sessions: [
    { id: "01J9ZK7M3Q8T5V2X4Y6Z0A1B2C", title: "Sessions list views" },
    { id: "01J9ZM1P3Q8T5V2X4Y6Z0A1B2D" },
  ],
  numbered: [
    { kind: "pull_request", number: "2219", title: "#2219 Pin the Bridge store clock" },
    { kind: "issue", number: "2214", title: "#2214 Sessions list drops a quiet Session" },
    { kind: "pull_request", number: "2207", title: "#2207 Stop the Drone count drifting" },
  ],
};

const FORGED: RefWorld = { ...WORLD, numbered: [{ kind: "pull_request", number: "12", title: "#12 Round once", url: "https://github.com/o/r/pull/12" }] };

describe("the forge's address for one", () => {
  const found = (text: string) => scanRefs(text, FORGED).map((one) => text.slice(one.from, one.to));
  it("is the reference, where it is this repository's", () => {
    expect(found("See https://github.com/o/r/pull/12.")).toEqual(["https://github.com/o/r/pull/12"]);
  });
  it("is left alone for another repository, a page of it, or a link already written", () => {
    expect(found("https://github.com/o/other/pull/12 and https://github.com/o/r/pull/12/files")).toEqual([]);
    expect(found("[the pull](https://github.com/o/r/pull/12)")).toEqual([]);
  });
});

const words = (text: string) => scanRefs(text, WORLD).map((one) => [text.slice(one.from, one.to), one.ref.kind, one.ref.id]);

describe("a number", () => {
  it("links a pull request and an issue the repository holds, and leaves any other number as text", () => {
    expect(words("#2219 fixes #2214, and #9999 is nothing")).toEqual([
      ["#2219", "pull_request", "2219"],
      ["#2214", "issue", "2214"],
    ]);
  });

  it("carries the title without the number it leads with", () => {
    const [one] = scanRefs("see #2219", WORLD);
    expect(one?.ref).toEqual({ kind: "pull_request", id: "2219", title: "Pin the Bridge store clock" });
  });

  it("is not read inside an address, a word or another reference", () => {
    expect(words("https://forge.invalid/pull/2219 and abc#2219 and &#2219; and ##2219")).toEqual([]);
  });
});

describe("a Session", () => {
  it("is linked by its id, its address and its exact title", () => {
    expect(words("Sessions list views, or s-01J9ZK7M, or 01J9ZK7M3Q8T5V2X4Y6Z0A1B2C")).toEqual([
      ["Sessions list views", "session", "01J9ZK7M3Q8T5V2X4Y6Z0A1B2C"],
      ["s-01J9ZK7M", "session", "01J9ZK7M3Q8T5V2X4Y6Z0A1B2C"],
      ["01J9ZK7M3Q8T5V2X4Y6Z0A1B2C", "session", "01J9ZK7M3Q8T5V2X4Y6Z0A1B2C"],
    ]);
  });

  it("names a Session with no title by its id", () => {
    const [one] = scanRefs("s-01J9ZM1P", WORLD);
    expect(one?.ref).toEqual({ kind: "session", id: "01J9ZM1P3Q8T5V2X4Y6Z0A1B2D", title: "01J9ZM1P3Q8T5V2X4Y6Z0A1B2D" });
  });
});

const row = (branch: string, number?: number) => ({ branch, state: "landed", ...(number === undefined ? {} : { pull_request: { number, url: `https://forge.invalid/pull/${number}` } }) });

/** One repository's line, as Fleet answers it: rows in each list, and the hub's open and merged pull requests. */
const LINE: MergeLine = {
  root: "/repos/armada",
  line: [row("fix/queued", 301)],
  off: [],
  landed: [row("fix/landed", 302)],
  sent_back: [row("fix/sent-back", 303)],
  hub: {
    pull_requests: [{ number: 2219, title: "Pin the Bridge store clock", branch: "fix/store-clock", url: "https://forge.invalid/pull/2219" }],
    merged: [{ number: 2207, title: "Stop the Drone count drifting", branch: "fix/drone-count", url: "https://forge.invalid/pull/2207", merged_at: "2026-10-11T10:00:00Z" }],
  },
};

describe("where a pull request opens on the merge line", () => {
  it("is its branch, from an open pull request, a merged one, or a row of the line", () => {
    expect(branchOfPullRequest(LINE, "2219")).toBe("fix/store-clock");
    expect(branchOfPullRequest(LINE, "2207")).toBe("fix/drone-count");
    expect(branchOfPullRequest(LINE, "301")).toBe("fix/queued");
    expect(branchOfPullRequest(LINE, "302")).toBe("fix/landed");
    expect(branchOfPullRequest(LINE, "303")).toBe("fix/sent-back");
  });

  it("is nothing where the line has no row for it, so the line opens with nothing marked", () => {
    expect(branchOfPullRequest(LINE, "2190")).toBeUndefined();
    expect(branchOfPullRequest({ ...LINE, hub: undefined }, "2219")).toBeUndefined();
    expect(branchOfPullRequest({ root: "/repos/armada", line: [], off: [], landed: [], sent_back: [] }, "2219")).toBeUndefined();
    expect(branchOfPullRequest(undefined, "2219")).toBeUndefined();
  });
});
