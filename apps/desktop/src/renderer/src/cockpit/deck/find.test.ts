// What the list's search finds: a Session as the Sessions page found it, a Job by its title, id, handle or branch, a blank search
// narrowing nothing, and a row saying what matched in place of its age only where it was not the title.

import { expect, test } from "vitest";
import type { JobSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";

import { foundOf } from "./find";

const session = (id: string, rest: Partial<Session> = {}): Session => ({ id, attachments: [], ...rest }) as Session;
const job = (id: string, rest: Partial<JobSummary> = {}): JobSummary => ({ id, handle: `${id}-handle`, title: id, ...rest }) as JobSummary;

const pull = { kind: "pull_request", number: 1702, title: "Backfill", branch: "chore/audit-backfill", address: "a", checks: {} as never, state: "merged", auto: false } as const;
const sessions = [
  session("s1", { title: "Backfill the audit log", attachments: [pull] }),
  session("s2", { title: "Trim the retry loop", attachments: [{ kind: "branch", name: "fix/retry-loop", slot: 3 }, { kind: "slot", slot: 3 }] }),
  session("s3", { title: "Draft the notes" }),
];
const jobs = [job("j1", { title: "The retry loop", handle: "52-the-retry-loop", branch: "fix/retry-loop" }), job("j2", { title: "Round the totals", handle: "7-round-totals", branch: "fix/totals" })];

const keys = (query: string) => [...(foundOf(query, sessions, jobs)?.keys() ?? [])].sort();

test("a blank search finds nothing in particular and narrows nothing", () => {
  expect(foundOf("", sessions, jobs)).toBeUndefined();
  expect(foundOf("   ", sessions, jobs)).toBeUndefined();
});

test("a Session is found by its title, whatever the case", () => {
  expect(keys("AUDIT")).toEqual(["session:s1"]);
  expect(foundOf("audit", sessions, jobs)!.get("session:s1")).toEqual({ short: "", whole: "" });
});

test("a Session is found by a pull request number and the row says which", () => {
  expect(keys("#1702")).toEqual(["session:s1"]);
  expect(foundOf("#1702", sessions, jobs)!.get("session:s1")).toEqual({ short: "#1702", whole: "Pull request #1702" });
});

test("a Session is found by its branch and by its slot", () => {
  expect(foundOf("fix/retry", sessions, [])?.get("session:s2")).toEqual({ short: "fix/retry-loop", whole: "Branch fix/retry-loop" });
  expect(foundOf("slot 3", sessions, [])?.get("session:s2")).toEqual({ short: "slot 3", whole: "Worktree slot 3" });
});

test("a Job is found by its title, its handle or its branch", () => {
  expect(keys("round the")).toEqual(["job:j2"]);
  expect(foundOf("round the", sessions, jobs)!.get("job:j2")).toEqual({ short: "", whole: "" });
  expect(foundOf("7-round", sessions, jobs)!.get("job:j2")).toEqual({ short: "7-round-totals", whole: "Job 7-round-totals" });
  expect(foundOf("fix/totals", sessions, jobs)!.get("job:j2")).toEqual({ short: "fix/totals", whole: "Branch fix/totals" });
});

test("one search can find both a Session and a Job", () => {
  expect(keys("retry")).toEqual(["job:j1", "session:s2"]);
});

test("a search that matches nothing is an empty set, not a blank one", () => {
  expect(foundOf("zzz", sessions, jobs)?.size).toBe(0);
});
