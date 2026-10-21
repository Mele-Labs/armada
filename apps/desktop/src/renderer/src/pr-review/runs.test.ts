// What a run comes to, in the shapes the surface draws: each result shape mapped, a change's hunk cut from its file's patch with the
// forge's numbering kept, and a finding's id fixed by the run's key and its place in the answer. Nothing here touches Fleet.

import type { ReviewResult, ReviewRun } from "@armada/protocol";
import { expect, test } from "vitest";

import type { PatchRead } from "./model";
import { counted, linesOfPatch, placeOfLine } from "./patch";
import { accountOf, agentRunOf, answerOf, areaShown, explanationOf, findingsOf, matchesOf, placeSaid, placeWords, runKey, searchId, searchName, toolDefOf, walkOf, wordsOf } from "./runs";

const PATCH = ["@@ -10,6 +10,7 @@", " a", " b", " c", "-d", "+D", "+E", " f", " g", "@@ -40,3 +41,3 @@", " x", "-y", "+Y", " z"].join("\n");
const patches = (): Record<string, PatchRead> => ({ "a.rs": { state: "read", lines: linesOfPatch("a.rs", PATCH) } });
const themes = (at: { start: number; end: number; side?: "left" | "right"; path?: string }): ReviewResult => ({
  shape: "themes",
  summary: "S",
  themes: [{ title: "T", purpose: "P", changes: [{ title: "C", why: "W", at: { path: "a.rs", side: "right", ...at } }] }],
});

test("each shape of a result maps to the account the surface draws", () => {
  expect(accountOf({ shape: "impact", rows: [{ what: "w", before: "b", after: "a" }] })).toEqual({ shape: "change", rows: [{ what: "w", before: "b", after: "a" }] });
  expect(accountOf({ shape: "reason", problem: "p", cost: "c" })).toEqual({ shape: "reason", problem: "p", cost: "c", links: [] });
  expect(accountOf({ shape: "text", body: "Words" })).toEqual({ shape: "text", body: "Words" });
  expect(accountOf({ shape: "steps", steps: [{ action: "Run it", look: "It passes", code: "make", source: { from: "doc", path: "docs/a.md" } }, { action: "Open it", look: "It opens", source: { from: "description" } }] })).toEqual({
    shape: "steps",
    steps: [
      { do: "Run it", look: "It passes", code: "make", from: { from: "doc", path: "docs/a.md", address: "docs/a.md" } },
      { do: "Open it", look: "It opens", from: { from: "description" } },
    ],
  });
  expect(accountOf({ shape: "findings", findings: [] })).toBeUndefined();
  expect(accountOf(undefined)).toBeUndefined();
});

test("an answer or an account of no words is none", () => {
  expect(answerOf({ shape: "text", body: "" })).toBeUndefined();
  expect(accountOf({ shape: "text", body: "" })).toBeUndefined();
  expect(answerOf({ shape: "text", body: "Yes." })).toBe("Yes.");
});

test("a change has no hunk until its file's patch is read, and then only the lines around its place", () => {
  expect(explanationOf(themes({ start: 11, end: 12 }), {}).themes[0]!.changes[0]).toMatchObject({ id: "t1c1", title: "C", why: "W", hunks: [] });
  const change = explanationOf(themes({ start: 13, end: 14 }), patches()).themes[0]!.changes[0]!;
  const hunk = change.hunks[0]!;
  expect(hunk.path).toBe("a.rs");
  // Three lines of context either side, inside the hunk the place is in; the second hunk is left out.
  expect(hunk.lines.map((one) => one.text)).toEqual(["@@ -11,5 +11,6 @@", " b", " c", "-d", "+D", "+E", " f", " g"]);
});

test("a cut keeps the forge's numbering, so a line is still named by its place in the file", () => {
  const hunk = explanationOf(themes({ start: 14, end: 14 }), patches()).themes[0]!.changes[0]!.hunks[0]!;
  expect(hunk.lines[0]!.text).toBe("@@ -12,4 +12,5 @@");
  const at = hunk.lines.findIndex((one) => one.text === "+E");
  expect(placeOfLine(hunk.lines, at)).toEqual({ line: 14, side: "right" });
  expect(placeOfLine(hunk.lines, hunk.lines.findIndex((one) => one.text === "-d"))).toEqual({ line: 13, side: "left" });
  expect(counted(hunk.lines).filter((one) => one.now !== undefined)).toHaveLength(5);
});

test("a place on the base's side is found by the base's numbers, and a place the patch lacks returns the patch whole", () => {
  const left = explanationOf(themes({ start: 13, end: 13, side: "left" }), patches()).themes[0]!.changes[0]!.hunks[0]!;
  expect(left.lines.some((one) => one.text === "-d")).toBe(true);
  const lost = explanationOf(themes({ start: 900, end: 901 }), patches()).themes[0]!.changes[0]!.hunks[0]!;
  expect(lost.lines).toEqual(linesOfPatch("a.rs", PATCH));
});

test("a place across two hunks keeps both headers", () => {
  const hunk = explanationOf(themes({ start: 15, end: 42 }), patches()).themes[0]!.changes[0]!.hunks[0]!;
  expect(hunk.lines.filter((one) => one.kind === "hunk").map((one) => one.text)).toEqual(["@@ -13,6 +13,7 @@", "@@ -40,3 +41,3 @@"]);
});

test("a result that is not themes explains nothing", () => {
  expect(explanationOf({ shape: "text", body: "x" }, patches())).toEqual({ summary: "", themes: [] });
  expect(explanationOf(undefined, {})).toEqual({ summary: "", themes: [] });
});

const found: ReviewRun = {
  manifest_id: "m1",
  number: 1,
  kind: "tool",
  name: "risks",
  head: "h",
  state: "done",
  started_at: "2026-10-16T10:00:00Z",
  result: { shape: "findings", findings: [{ title: "One", body: "b", severity: "high", at: { path: "a.rs", side: "right", start: 3, end: 5 } }, { title: "Two", body: "c", severity: "low", at: { path: "b.rs", side: "left", start: 9, end: 9 } }] },
};

test("a finding's id is the run's key and its place in the answer, and none is put away or sent yet", () => {
  expect(runKey("tool", "risks")).toBe("tool:risks");
  expect(findingsOf(found).map((one) => [one.id, one.tool, one.severity, one.dismissed, one.sent])).toEqual([
    ["tool:risks:0", "risks", "high", false, false],
    ["tool:risks:1", "risks", "low", false, false],
  ]);
  expect(findingsOf({ ...found, result: { shape: "text", body: "" } })).toEqual([]);
});

test("a place is said as path and line, or path and lines", () => {
  expect(placeSaid({ path: "a.rs", start: 3, end: 3 })).toBe("a.rs:3");
  expect(placeSaid({ path: "a.rs", start: 3, end: 5 })).toBe("a.rs:3-5");
});

test("a run's state is going, kept for its commit, or failed in Fleet's words", () => {
  expect(agentRunOf({ ...found, state: "queued" })).toMatchObject({ state: "running" });
  expect(agentRunOf({ ...found, state: "running" })).toMatchObject({ state: "running", since: Date.parse("2026-10-16T10:00:00Z") });
  expect(agentRunOf({ ...found, finished_at: "2026-10-16T10:00:09Z" })).toEqual({ state: "done", at: Date.parse("2026-10-16T10:00:09Z"), head: "h" });
  expect(agentRunOf({ ...found, state: "failed", failed: "The answer was not JSON" })).toMatchObject({ state: "failed", said: "The answer was not JSON", head: "h" });
});

test("Armada's three tools have a mark each and any other definition takes the clipboard", () => {
  const def = (id: string) => toolDefOf({ id, kind: "tool", name: id, prompt: "p", source: "armada" });
  expect([def("risks").mark, def("possible-bugs").mark, def("api-breaking-changes").mark, def("test-gaps").mark]).toEqual(["risks", "bugs", "breaks", "custom"]);
  expect(toolDefOf({ id: "x", kind: "explanation", name: "X", prompt: "p", source: "kit", replaces: "armada" })).toMatchObject({ agent: true, checks: "p", replaces: "armada" });
});

test("a search is named by his words, lower cased and single spaced, behind a prefix no list shares", () => {
  expect(wordsOf("  PRs that   touch Fleet ")).toBe("PRs that touch Fleet");
  expect(searchName("  PRs that   touch Fleet ")).toBe("prs that touch fleet");
  expect(searchId(searchName("Billing PRs"))).toBe("search:billing prs");
});

test("only a finished search has matches", () => {
  const pull = { manifest_id: "m1", repository: "o/r", number: 7, title: "T", branch: "b", address: "a", draft: false, checks: "passed", review: "none", head: "h", author: "x", updated_at: "2026-10-16T10:00:00Z" } as const;
  const base: ReviewRun = { manifest_id: "*", number: 0, kind: "search", name: "billing", head: "", state: "done", started_at: "2026-10-16T10:00:00Z", result: { shape: "matches", matches: [{ pull, reason: "touches billing" }] } };
  expect(matchesOf(base)).toHaveLength(1);
  expect(matchesOf({ ...base, state: "running" })).toEqual([]);
  expect(matchesOf({ ...base, result: { shape: "text", body: "" } })).toEqual([]);
});

test("a walk is the finished run's web or API walk, with the place it was written against", () => {
  const place = { how: "deployed", address: "https://x.test" } as const;
  const web: ReviewRun = { manifest_id: "m1", number: 1, kind: "tool", name: "walk", head: "h", state: "done", started_at: "2026-10-16T10:00:00Z", result: { shape: "web_walk", place, steps: [] } };
  expect(walkOf(web)?.place).toEqual(place);
  expect(walkOf({ ...web, result: { shape: "api_walk", place, requests: [] } })?.shape).toBe("api_walk");
  expect(walkOf({ ...web, state: "running" })).toBeUndefined();
  expect(walkOf({ ...web, result: { shape: "findings", findings: [] } })).toBeUndefined();
  expect(walkOf(undefined)).toBeUndefined();
});

test("a place is said as where it is and what starts it", () => {
  expect(placeWords({ how: "deployed", address: "https://x.test" })).toBe("Deployed at https://x.test");
  expect(placeWords({ how: "deployed", address: "https://x.test", api: "https://api.test" })).toBe("Deployed at https://x.test, API at https://api.test");
  expect(placeWords({ how: "pull", command: "pnpm dev", address: "http://localhost:5173" })).toBe("Pull it down, run pnpm dev, at http://localhost:5173");
  expect(placeWords({ how: "pull", command: "armada fleet --dev", api: "http://localhost:7420" })).toBe("Pull it down, run armada fleet --dev, API at http://localhost:7420");
  expect(areaShown("crates/fleet")).toBe("crates/fleet/");
  expect(areaShown("crates/fleet/")).toBe("crates/fleet/");
});

test("a Kit tool that hides the repository's keeps the mark", () => {
  expect(toolDefOf({ id: "x", kind: "tool", name: "X", prompt: "p", source: "kit", hides: "repository" })).toMatchObject({ source: "kit", hides: "repository" });
  expect(toolDefOf({ id: "x", kind: "tool", name: "X", prompt: "p", source: "kit" })).not.toHaveProperty("hides");
});
