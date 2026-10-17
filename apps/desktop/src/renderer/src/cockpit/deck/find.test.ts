// What the list's search finds: a Session as the Sessions page found it, a blank search
// narrowing nothing, and a row saying what matched in place of its age only where it was not the title.

import { expect, test } from "vitest";
import type { Session } from "@armada/screens/src/draft/sessions";

import { foundOf } from "./find";

const session = (id: string, rest: Partial<Session> = {}): Session => ({ id, attachments: [], ...rest }) as Session;

const pull = { kind: "pull_request", number: 1702, title: "Backfill", branch: "chore/audit-backfill", address: "a", checks: {} as never, state: "merged", auto: false } as const;
const sessions = [
  session("s1", { title: "Backfill the audit log", attachments: [pull] }),
  session("s2", { title: "Trim the retry loop", attachments: [{ kind: "branch", name: "fix/retry-loop", slot: 3 }, { kind: "slot", slot: 3 }] }),
  session("s3", { title: "Draft the notes" }),
];

const keys = (query: string) => [...(foundOf(query, sessions)?.keys() ?? [])].sort();

test("a blank search finds nothing in particular and narrows nothing", () => {
  expect(foundOf("", sessions)).toBeUndefined();
  expect(foundOf("   ", sessions)).toBeUndefined();
});

test("a Session is found by its title, whatever the case", () => {
  expect(keys("AUDIT")).toEqual(["session:s1"]);
  expect(foundOf("audit", sessions)!.get("session:s1")).toEqual({ short: "", whole: "" });
});

test("a Session is found by a pull request number and the row says which", () => {
  expect(keys("#1702")).toEqual(["session:s1"]);
  expect(foundOf("#1702", sessions)!.get("session:s1")).toEqual({ short: "#1702", whole: "Pull request #1702" });
});

test("a Session is found by its branch and by its slot", () => {
  expect(foundOf("fix/retry", sessions)?.get("session:s2")).toEqual({ short: "fix/retry-loop", whole: "Branch fix/retry-loop" });
  expect(foundOf("slot 3", sessions)?.get("session:s2")).toEqual({ short: "slot 3", whole: "Worktree slot 3" });
});

test("a search that matches nothing is an empty set, not a blank one", () => {
  expect(foundOf("zzz", sessions)?.size).toBe(0);
});
