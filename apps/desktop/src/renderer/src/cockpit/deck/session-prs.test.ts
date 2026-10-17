import { describe, expect, it } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { sessionHolds } from "./session-prs";

const pull = (number: number, branch: string, owner?: { kind: "session"; id: string; title: string }) => ({ number, branch, ...(owner === undefined ? {} : { owner }) });
const attached = (number: number, branch: string): Session["attachments"][number] => ({ kind: "pull_request", number, title: "t", branch, address: "a", checks: { state: "passed" }, state: "draft", auto: false });

describe("the pull requests a Session holds", () => {
  it("holds one it owns, one it attached by number or branch, and one on its branch", () => {
    const session = { id: "s1", attachments: [attached(10, "fix/a"), { kind: "branch" as const, name: "fix/b", slot: 1 }] };
    expect(sessionHolds(session, pull(99, "x", { kind: "session", id: "s1", title: "" }))).toBe(true);
    expect(sessionHolds(session, pull(10, "other"))).toBe(true);
    expect(sessionHolds(session, pull(11, "fix/a"))).toBe(true);
    expect(sessionHolds(session, pull(12, "fix/b"))).toBe(true);
  });

  it("does not hold another Session's", () => {
    const session = { id: "s1", attachments: [attached(10, "fix/a")] };
    expect(sessionHolds(session, pull(20, "fix/c", { kind: "session", id: "s2", title: "" }))).toBe(false);
    expect(sessionHolds(session, pull(22, "fix/e"))).toBe(false);
  });
});
