// The review store over a fake source: the lists merged by pull request, a list the forge would not answer keeping what
// it held, a page read when a pull request is opened, a patch read when its change is shown, and a review posted as one
// call with every comment placed by path, line and side. The agent half is runs Fleet keeps: read when a pull request is opened,
// moved by `review_run.changed`, started only on a press, and shown failed in Fleet's words. Natural Search is a run across every
// repository, kept under his words, and the Walk finds where a change runs before it writes a walk. Nothing here touches `window.armada`.

import { forgetAllKept, writeKept } from "@armada/components";
import type { PullRequestPage, PullReviewLists, PullReviewRow, RememberedPlace, ReviewPlace, ReviewRun, ReviewTools, ReviewWalk, StartReviewRun, StartReviewSearch, WalkPlaceFound, WritePullRequest } from "@armada/protocol";
import { beforeEach, expect, test } from "vitest";

import type { ToolDef } from "./model";
import type { ReviewWalkNote, ShownReviewWalk } from "../../../shared/review-walk";
import type { Read, ReviewSource } from "./source";
import { createReviewStore } from "./store";
import { COMMENTS_TOOL } from "./tools";

const row = (number: number, at: string, over: Partial<PullReviewRow> = {}): PullReviewRow => ({
  manifest_id: "m1",
  repository: "Mele-Labs/armada",
  number,
  title: `Pull request ${number}`,
  branch: `branch-${number}`,
  address: `https://example.test/${number}`,
  draft: false,
  checks: "passed",
  review: "none",
  head: `head${number}`,
  author: "devon",
  updated_at: at,
  ...over,
});

const lists = (over: Partial<PullReviewLists> = {}): PullReviewLists => ({ me: "nick", fetched_at: "2026-10-16T12:00:00Z", requested: [], team: [], assigned: [], mine: [], ...over });

const page = (number: number, over: Partial<PullRequestPage> = {}): PullRequestPage => ({
  manifest_id: "m1",
  number,
  base: "main",
  author: "devon",
  additions: 3,
  deletions: 1,
  mergeable: "mergeable",
  review: "none",
  approvals: 0,
  conversation: [],
  checks: [],
  files: [],
  commits: [],
  head: `head${number}`,
  body: "The description",
  ...over,
});

const said = (words: string): Read<never> => ({ ok: false, said: words });

type Calls = {
  writes: WritePullRequest[];
  resolves: { thread: string; resolved: boolean }[];
  files: string[];
  starts: StartReviewRun[];
  saves: string[];
  searches: StartReviewSearch[];
  forgotten: string[];
  saved: { manifestId: string; area: string }[];
  opened: { number: number; how: string }[];
  shown: ShownReviewWalk[];
  emit: (run: ReviewRun) => void;
  walk: (walk: ReviewWalk) => void;
  note: (note: ReviewWalkNote) => void;
  closed: (walkId: string) => void;
};

const TOOLS: ReviewTools = {
  tools: [
    { id: "risks", kind: "tool", name: "Risks", prompt: "What could go wrong", source: "armada" },
    { id: "api-breaking-changes", kind: "tool", name: "API breaking changes", prompt: "Public shapes", source: "armada" },
    { id: "how-it-works", kind: "explanation", name: "How it works", prompt: "How the code does it", source: "armada" },
    { id: "why", kind: "explanation", name: "Why", prompt: "The problem", source: "armada", replaces: "armada" },
  ],
  left_out: [{ file: "/kit/notes.md", source: "kit", said: "No kind in its front matter" }],
};

const run = (over: Partial<ReviewRun> = {}): ReviewRun => {
  const state = over.state ?? "done";
  const going = state === "queued" || state === "running";
  return { manifest_id: "m1", number: 1, kind: "tool", name: "risks", head: "head1", state, started_at: "2026-10-16T10:00:00Z", ...(going ? {} : { finished_at: "2026-10-16T10:00:05Z" }), ...over };
};
const search = (words: string, over: Partial<ReviewRun> = {}): ReviewRun => ({ manifest_id: "*", number: 0, kind: "search", name: words.trim().replace(/\s+/g, " ").toLowerCase(), head: "", state: "done", started_at: "2026-10-16T10:00:00Z", finished_at: "2026-10-16T10:00:09Z", ...over });
const place = (start: number, end = start, over: Partial<ReviewPlace> = {}): ReviewPlace => ({ path: "a.rs", side: "right", start, end, ...over });

function fake(over: Partial<ReviewSource> = {}): { source: ReviewSource; calls: Calls } {
  let told: (run: ReviewRun) => void = () => undefined;
  let walked: (walk: ReviewWalk) => void = () => undefined;
  let noted: (note: ReviewWalkNote) => void = () => undefined;
  let closed: (walkId: string) => void = () => undefined;
  const calls: Calls = {
    writes: [],
    resolves: [],
    files: [],
    starts: [],
    saves: [],
    searches: [],
    forgotten: [],
    saved: [],
    opened: [],
    shown: [],
    emit: (one) => told(one),
    walk: (one) => walked(one),
    note: (one) => noted(one),
    closed: (one) => closed(one),
  };
  const source: ReviewSource = {
    lists: async () => ({ ok: true, value: lists() }),
    page: async (_id, number) => ({ ok: true, value: page(number) }),
    file: async (_id, number, path) => {
      calls.files.push(path);
      return { ok: true, value: { manifest_id: "m1", number, path, patch: "@@ -1,2 +1,2 @@\n keep\n-old\n+new" } };
    },
    write: async (_id, write) => {
      calls.writes.push(write);
      return { ok: true, value: page(write.pull_request, { review: write.kind === "approve" ? "approved" : "none", threads: [] }) };
    },
    resolve: async (_id, resolve) => {
      calls.resolves.push(resolve);
      return { ok: true, value: resolve };
    },
    tools: async () => ({ ok: true, value: TOOLS }),
    saveTool: async (_id, save) => {
      calls.saves.push(`${save.scope}:${save.kind}:${save.name}`);
      return { ok: true, value: { tools: [...TOOLS.tools, { id: "test-gaps", kind: "tool", name: save.name, prompt: save.prompt, source: save.scope }] } };
    },
    startRun: async (_id, start) => {
      calls.starts.push(start);
      return { ok: true, value: run({ kind: start.kind, name: start.name, state: "queued" }) };
    },
    runs: async () => ({ ok: true, value: { runs: [] } }),
    onRun: (on) => {
      told = on;
      return () => undefined;
    },
    startSearch: async (start) => {
      calls.searches.push(start);
      return { ok: true, value: search(start.words, { state: "queued" }) };
    },
    searches: async () => ({ ok: true, value: { runs: [] } }),
    forgetSearch: async (forget) => {
      calls.forgotten.push(forget.words);
      return { ok: true, value: { runs: [] } };
    },
    findPlace: async () => ({ ok: true, value: { area: "crates/fleet", tried: [] } }),
    places: async () => ({ ok: true, value: { places: [] } }),
    savePlace: async (manifestId, save) => {
      calls.saved.push({ manifestId, area: save.area });
      return { ok: true, value: { id: "p1", manifest_id: manifestId, repository: "Mele-Labs/armada", area: save.area, place: save.place } };
    },
    forgetPlace: async () => ({ ok: true, value: { places: [] } }),
    openWalk: async (_id, open) => {
      calls.opened.push({ number: open.number, how: open.place.how });
      return { ok: true, value: { id: "w1", manifest_id: "m1", number: open.number, state: open.place.how === "pull" ? "checking_out" : "ready", address: open.place.address ?? open.place.api ?? "" } };
    },
    closeWalk: async (_id, id) => ({ ok: true, value: { id, manifest_id: "m1", number: 1, state: "closed", address: "" } }),
    onWalk: (on) => {
      walked = on;
      return () => undefined;
    },
    showWalk: async (shown) => {
      calls.shown.push(shown);
      return { ok: true };
    },
    onNote: (on) => {
      noted = on;
      return () => undefined;
    },
    onClosed: (on) => {
      closed = on;
      return () => undefined;
    },
    ...over,
  };
  return { source, calls };
}

// A reading kept by one test would draw in the next store made.
beforeEach(forgetAllKept);

const settle = () => new Promise((done) => setTimeout(done, 0));

async function opened(over: Partial<ReviewSource> = {}) {
  const { source, calls } = fake({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z")] }) }), ...over });
  const store = createReviewStore(source);
  store.fetch();
  await settle();
  store.open("Mele-Labs/armada#1");
  await settle();
  return { store, calls, key: "Mele-Labs/armada#1" };
}

test("one pull request on two lists is one row that knows both, ordered by when the forge last changed it", async () => {
  const both = row(2, "2026-10-16T09:00:00Z");
  const { source } = fake({
    lists: async () => ({
      ok: true,
      value: lists({ requested: [both, row(1, "2026-10-16T08:00:00Z")], team: [{ ...both, team: "@Mele-Labs/bridge" }], mine: [row(3, "2026-10-16T11:00:00Z")] }),
    }),
  });
  const store = createReviewStore(source);
  store.fetch();
  await settle();
  const snapshot = store.get();
  expect(snapshot.prs.map((one) => one.number)).toEqual([3, 2, 1]);
  expect(snapshot.prs.find((one) => one.number === 2)?.lists).toEqual(["requested", "team"]);
  expect(snapshot.prs.find((one) => one.number === 2)?.team).toBe("@Mele-Labs/bridge");
  expect(snapshot.me).toBe("nick");
  expect(snapshot.fetching).toBe(false);
});

test("a list the forge would not answer keeps what it held, and the others are replaced", async () => {
  let reads = 0;
  const { source } = fake({
    lists: async () => {
      reads += 1;
      return { ok: true, value: reads === 1 ? lists({ requested: [row(1, "2026-10-16T08:00:00Z")], assigned: [row(2, "2026-10-16T09:00:00Z")] }) : lists({ silent: ["assigned"] }) };
    },
  });
  const store = createReviewStore(source);
  store.fetch();
  await settle();
  store.fetch();
  await settle();
  // Requested answered empty and lost its row. Assigned was silent and kept its own.
  expect(store.get().prs.map((one) => [one.number, one.lists])).toEqual([[2, ["assigned"]]]);
});

test("a store made after a reading was kept draws its rows and age before Fleet answers", () => {
  writeKept("pr-review:lists", lists({ me: "nick", fetched_at: "2026-10-16T12:00:00Z", requested: [row(1, "2026-10-16T10:00:00Z")] }));
  const store = createReviewStore(fake({ lists: () => new Promise(() => undefined) }).source);
  expect(store.get().prs.map((one) => one.number)).toEqual([1]);
  expect(store.get().me).toBe("nick");
  expect(store.get().fetchedAt).toBe(Date.parse("2026-10-16T12:00:00Z"));
  expect(store.get().fetching).toBe(false);
});

test("a reading that arrived is kept for the next store", async () => {
  const first = createReviewStore(fake({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z")] }) }) }).source);
  first.fetch();
  await settle();
  const next = createReviewStore(fake({ lists: () => new Promise(() => undefined) }).source);
  expect(next.get().prs.map((one) => one.number)).toEqual([1]);
});

test("a refused read says why and leaves the lists as they were", async () => {
  let reads = 0;
  const { source } = fake({ lists: async () => (reads++ === 0 ? { ok: true, value: lists({ requested: [row(1, "2026-10-16T08:00:00Z")] }) } : said("gh is not signed in")) });
  const store = createReviewStore(source);
  store.fetch();
  await settle();
  store.fetch();
  await settle();
  expect(store.get().failed).toBe("gh is not signed in");
  expect(store.get().prs).toHaveLength(1);
  expect(store.get().fetching).toBe(false);
});

test("opening a pull request reads its page, its threads and its description", async () => {
  const thread = { id: "T1", path: "a.rs", line: 12, side: "right" as const, resolved: false, comments: [{ id: "C1", author: "devon", at: "2026-10-16T09:00:00Z", body: "Why here?" }] };
  const { store, key } = await opened({ page: async (_id, number) => ({ ok: true, value: page(number, { threads: [thread], body: "Holds the read" }) }) });
  const work = store.get().work[key]!;
  expect(work.page.state).toBe("read");
  expect(work.threads).toEqual([thread]);
  expect(store.get().prs[0]!.body).toBe("Holds the read");
});

test("a page that cannot be read says so, and it is not a blank pull request", async () => {
  const { store, key } = await opened({ page: async () => said("The forge did not answer") });
  expect(store.get().work[key]!.page).toEqual({ state: "said", said: "The forge did not answer" });
});

test("a file's patch is read once, the first time its change is shown", async () => {
  const { store, calls, key } = await opened();
  store.readFile(key, "a.rs");
  store.readFile(key, "a.rs");
  await settle();
  expect(calls.files).toEqual(["a.rs"]);
  const read = store.get().work[key]!.patches["a.rs"];
  expect(read?.state).toBe("read");
});

test("Submit posts the drafts as one review: a reply, a line, a file, and the commit he read", async () => {
  const { store, calls, key } = await opened();
  store.addDraft(key, { path: "a.rs", line: 12, side: "right", body: "Take the lock after the call" });
  store.addDraft(key, { path: "a.rs", line: 4, side: "left", body: "This went" });
  store.addDraft(key, { path: "b.rs", body: "Whole file note" });
  store.addDraft(key, { path: "a.rs", body: "Agreed", thread: "T1" });
  const taken = await store.submit(key, "request_changes", "  The lock first  ");
  expect(taken).toBe(true);
  expect(calls.writes).toEqual([
    {
      pull_request: 1,
      kind: "request_changes",
      body: "The lock first",
      commit: "head1",
      comments: [
        { kind: "line", path: "a.rs", line: 12, side: "right", body: "Take the lock after the call" },
        { kind: "line", path: "a.rs", line: 4, side: "left", body: "This went" },
        { kind: "file", path: "b.rs", body: "Whole file note" },
        { kind: "reply", thread: "T1", body: "Agreed" },
      ],
    },
  ]);
  const work = store.get().work[key]!;
  expect(work.drafts).toEqual([]);
  expect(work.submissions.map((one) => [one.verdict, one.comments])).toEqual([["request_changes", 4]]);
  expect(work.submitting).toBe(false);
});

test("an approval with nothing to say posts no body and no comments", async () => {
  const { store, calls, key } = await opened();
  await store.submit(key, "approve", "");
  expect(calls.writes[0]).toEqual({ pull_request: 1, kind: "approve", commit: "head1" });
  expect(store.get().work[key]!.review).toBe("approved");
});

test("a refused review keeps every draft and shows the forge's words", async () => {
  const { store, key } = await opened({ write: async () => said("Can not approve your own pull request") });
  store.addDraft(key, { path: "a.rs", line: 12, side: "right", body: "Keep me" });
  const taken = await store.submit(key, "approve", "Fine");
  expect(taken).toBe(false);
  const work = store.get().work[key]!;
  expect(work.drafts.map((one) => one.body)).toEqual(["Keep me"]);
  expect(work.refused).toBe("Can not approve your own pull request");
  expect(work.submissions).toEqual([]);
  expect(work.submitting).toBe(false);
});

test("a thread is resolved at the forge and then here, and a refusal changes nothing", async () => {
  const thread = { id: "T1", path: "a.rs", line: 12, side: "right" as const, resolved: false, comments: [] };
  const { store, calls, key } = await opened({ page: async (_id, number) => ({ ok: true, value: page(number, { threads: [thread] }) }) });
  expect(await store.resolve(key, "T1", true)).toBe(true);
  expect(calls.resolves).toEqual([{ thread: "T1", resolved: true }]);
  expect(store.get().work[key]!.threads[0]!.resolved).toBe(true);

  const refused = await opened({ page: async (_id, number) => ({ ok: true, value: page(number, { threads: [thread] }) }), resolve: async () => said("Not allowed to resolve") });
  expect(await refused.store.resolve(refused.key, "T1", true)).toBe(false);
  expect(refused.store.get().work[refused.key]!.threads[0]!.resolved).toBe(false);
  expect(refused.store.get().work[refused.key]!.refused).toBe("Not allowed to resolve");
});

const toolOf = (store: ReturnType<typeof createReviewStore>, id: string): ToolDef => {
  const found = [...(store.get().tools["m1"] ?? []), COMMENTS_TOOL].find((one) => one.id === id);
  if (found === undefined) throw new Error(`no tool ${id}`);
  return found;
};

test("the tools are read once for a repository, and Comments stays Bridge's own", async () => {
  const { store } = await opened();
  expect(store.get().tools["m1"]!.map((one) => one.id)).toEqual(["risks", "api-breaking-changes", "how-it-works", "why"]);
  expect(store.get().tools["m1"]!.find((one) => one.id === "risks")).toMatchObject({ mark: "risks", agent: true, source: "armada" });
  expect(store.get().tools["m1"]!.find((one) => one.id === "why")?.replaces).toBe("armada");
  expect(store.get().leftOut["m1"]).toEqual([{ file: "/kit/notes.md", source: "kit", said: "No kind in its front matter" }]);
  expect(COMMENTS_TOOL.agent).toBe(false);
});

test("a row's holder is the Session that holds the pull request", async () => {
  const { store } = await opened({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z", { holder: { id: "s1", title: "Fix the wait" } })] }) }) });
  expect(store.get().prs[0]!.holder).toEqual({ id: "s1", title: "Fix the wait" });
});

test("opening a pull request reads what is kept for it, so a result survives a restart", async () => {
  const themes = run({ kind: "explanation", name: "how-it-works", result: { shape: "themes", summary: "It caches the read", themes: [{ title: "Cache", purpose: "Hold it", changes: [{ title: "Serve", why: "Fewer calls", at: place(2) }] }] } });
  const found = run({ result: { shape: "findings", findings: [{ title: "Lock held", body: "Across the call", severity: "high", at: place(2) }] } });
  const asked = run({ kind: "ask", name: "What if Fleet restarts?", result: { shape: "text", body: "It is lost." } });
  const impact = run({ kind: "explanation", name: "user-impact", head: "old0000", result: { shape: "impact", rows: [{ what: "Meters", before: "Five seconds", after: "A minute" }] } });
  const { store, key } = await opened({ runs: async () => ({ ok: true, value: { runs: [found, themes, asked, impact] } }) });
  const work = store.get().work[key]!;
  expect(work.explains["how-it-works"]).toMatchObject({ state: "done", head: "head1" });
  expect(work.explains["user-impact"]).toMatchObject({ state: "done", head: "old0000" });
  expect(work.accounts["user-impact"]).toEqual({ shape: "change", rows: [{ what: "Meters", before: "Five seconds", after: "A minute" }] });
  expect(work.runs["risks"]).toMatchObject({ state: "done" });
  expect(work.findings.map((one) => [one.id, one.severity, one.dismissed])).toEqual([["tool:risks:0", "high", false]]);
  expect(work.asks.map((one) => [one.words, one.answer])).toEqual([["What if Fleet restarts?", "It is lost."]]);
  expect(work.kind).toBe("how-it-works");
  expect(store.get().prs[0]!.explanation.themes[0]!.changes[0]).toMatchObject({ id: "t1c1", title: "Serve", hunks: [] });
});

test("a change's hunk is cut from its file's patch once the patch is read", async () => {
  const themes = run({ kind: "explanation", name: "how-it-works", result: { shape: "themes", summary: "", themes: [{ title: "Cache", purpose: "Hold it", changes: [{ title: "Serve", why: "", at: place(2) }] }] } });
  const { store, key } = await opened({ runs: async () => ({ ok: true, value: { runs: [themes] } }) });
  store.readFile(key, "a.rs");
  await settle();
  const hunk = store.get().prs[0]!.explanation.themes[0]!.changes[0]!.hunks[0]!;
  expect(hunk.path).toBe("a.rs");
  expect(hunk.lines.map((one) => one.kind)).toEqual(["hunk", "context", "removed", "added"]);
  expect(hunk.lines[0]!.text).toBe("@@ -1,2 +1,2 @@");
});

test("a press starts a run only when nothing is kept, and Run again asks for another", async () => {
  const { store, calls, key } = await opened();
  const risks = toolOf(store, "risks");
  store.runTool(key, risks);
  expect(store.get().work[key]!.runs["risks"]?.state).toBe("running");
  expect(store.get().work[key]!.view).toBe("risks");
  await settle();
  expect(calls.starts).toEqual([{ number: 1, kind: "tool", name: "risks" }]);
  // Pressed again while it is going, nothing starts.
  store.runTool(key, risks);
  expect(calls.starts).toHaveLength(1);
  store.rerun(key, risks);
  store.rerun(key, risks);
  await settle();
  expect(calls.starts).toEqual([{ number: 1, kind: "tool", name: "risks" }]);
  calls.emit(run({ result: { shape: "findings", findings: [] } }));
  store.rerun(key, risks);
  await settle();
  expect(calls.starts.at(-1)).toEqual({ number: 1, kind: "tool", name: "risks", again: true });
});

test("a run that moves on the stream is replaced whole, a late start does not take a finished run back, and a dismissed finding stays put", async () => {
  const { store, calls, key } = await opened();
  const done = run({ started_at: "2026-10-16T10:00:00Z", result: { shape: "findings", findings: [{ title: "One", body: "", severity: "low", at: place(2) }, { title: "Two", body: "", severity: "medium", at: place(2) }] } });
  calls.emit(run({ state: "running" }));
  expect(store.get().work[key]!.runs["risks"]?.state).toBe("running");
  calls.emit(done);
  store.dismiss(key, "tool:risks:0");
  calls.emit(run({ state: "queued" }));
  expect(store.get().work[key]!.runs["risks"]?.state).toBe("done");
  calls.emit(done);
  expect(store.get().work[key]!.findings.map((one) => [one.id, one.dismissed])).toEqual([["tool:risks:0", true], ["tool:risks:1", false]]);
  // A run for another pull request is not this one's.
  calls.emit(run({ number: 99 }));
  expect(Object.keys(store.get().work)).toEqual([key]);
});

test("a failed run is Fleet's reason and never a result, and Run again starts another", async () => {
  const { store, calls, key } = await opened();
  calls.emit(run({ kind: "explanation", name: "how-it-works", state: "failed", failed: "The model answered something that was not themes" }));
  const work = store.get().work[key]!;
  expect(work.explains["how-it-works"]).toMatchObject({ state: "failed", said: "The model answered something that was not themes" });
  expect(store.get().prs[0]!.explanation.themes).toEqual([]);
  store.explainAgain(key, toolOf(store, "how-it-works"));
  await settle();
  expect(calls.starts).toEqual([{ number: 1, kind: "explanation", name: "how-it-works", again: true }]);
});

test("a run Fleet would not start is failed in Fleet's words", async () => {
  const { store, key } = await opened({ startRun: async () => said("No definition has that name") });
  store.runTool(key, toolOf(store, "risks"));
  await settle();
  expect(store.get().work[key]!.runs["risks"]).toMatchObject({ state: "failed", said: "No definition has that name" });
});

test("a question is asked under his words, answered by the run, and removing it keeps it off the page", async () => {
  const { store, calls, key } = await opened();
  store.ask(key, "  Does this   change anything? ");
  await settle();
  expect(calls.starts).toEqual([{ number: 1, kind: "ask", name: "Does this change anything?" }]);
  const [asked] = store.get().work[key]!.asks;
  expect(asked).toMatchObject({ words: "Does this change anything?", run: { state: "running" } });
  expect(store.get().work[key]!.kind).toBe(asked!.id);
  calls.emit(run({ kind: "ask", name: "Does this change anything?", result: { shape: "text", body: "No." } }));
  expect(store.get().work[key]!.asks[0]).toMatchObject({ answer: "No.", run: { state: "done" } });
  store.removeAsk(key, asked!.id);
  calls.emit(run({ kind: "ask", name: "Does this change anything?", started_at: "2026-10-16T10:01:00Z", result: { shape: "text", body: "No." } }));
  expect(store.get().work[key]!.asks).toEqual([]);
});

test("saving a tool writes it at Fleet and reads the tools again, and a refusal is Fleet's words", async () => {
  const { store, calls } = await opened();
  const saved = await store.addTool({ kind: "tool", name: "Test gaps", checks: "What no test would notice", scope: "kit", manifestId: "m1" });
  expect(saved).toBeUndefined();
  expect(calls.saves).toEqual(["kit:tool:Test gaps"]);
  expect(store.get().tools["m1"]!.map((one) => one.id)).toContain("test-gaps");

  const refused = await opened({ saveTool: async () => said("test-gaps.md is already in the Kit") });
  expect(await refused.store.addTool({ kind: "tool", name: "Test gaps", checks: "x", scope: "kit", manifestId: "m1" })).toBe("test-gaps.md is already in the Kit");
});

test("a commit pushed since a result was kept leaves the result held with the commit it ran on", async () => {
  const kept = run({ head: "old0000", result: { shape: "findings", findings: [] } });
  const { store, key } = await opened({ runs: async () => ({ ok: true, value: { runs: [kept] } }) });
  expect(store.get().work[key]!.runs["risks"]).toMatchObject({ state: "done", head: "old0000" });
  expect(store.get().prs[0]!.head).toBe("head1");
});

const FOUND_ELSEWHERE = row(7, "2026-10-16T07:00:00Z", { manifest_id: "m2", repository: "Mele-Labs/pocket" });
const matched = (words: string, state: ReviewRun["state"] = "done") =>
  search(words, { state, result: { shape: "matches", matches: [{ pull: FOUND_ELSEWHERE, reason: "touches src/lib.rs, which you changed on 14 Oct" }] } });

test("Natural Search runs once across every repository, is kept under his words, and its matches open like any pull request", async () => {
  const { source, calls } = fake({ page: async (_id, number) => ({ ok: true, value: page(number, { body: "Found by a search" }) }) });
  const store = createReviewStore(source);
  store.find("  PRs that touch   files I've touched ");
  const id = "search:prs that touch files i've touched";
  expect(store.get().tab).toBe(id);
  expect(store.get().finds[0]).toMatchObject({ id, words: "PRs that touch files I've touched", run: { state: "running" }, hits: [] });
  await settle();
  expect(calls.searches).toEqual([{ words: "PRs that touch files I've touched" }]);
  // Progress arrives as a run of kind search, which has no pull request to look up.
  calls.emit(matched("PRs that touch files I've touched"));
  const find = store.get().finds[0]!;
  expect(find.run.state).toBe("done");
  expect(find.hits).toEqual([{ key: "Mele-Labs/pocket#7", reason: "touches src/lib.rs, which you changed on 14 Oct" }]);
  // The pull request is on none of the four lists and opens all the same.
  expect(store.get().prs.map((one) => [one.key, one.lists])).toEqual([["Mele-Labs/pocket#7", []]]);
  store.open("Mele-Labs/pocket#7");
  await settle();
  expect(store.get().work["Mele-Labs/pocket#7"]!.page.state).toBe("read");
  expect(store.get().prs[0]!.body).toBe("Found by a search");
});

test("a search kept by Fleet comes back with the lists, under its kept name until he writes it again, and is never marked stale", async () => {
  const { source } = fake({ searches: async () => ({ ok: true, value: { runs: [matched("Billing PRs")] } }) });
  const store = createReviewStore(source);
  store.fetch();
  await settle();
  expect(store.get().finds.map((one) => [one.id, one.words, one.run.state])).toEqual([["search:billing prs", "billing prs", "done"]]);
  // A newer commit anywhere changes nothing about it.
  store.fetch();
  await settle();
  expect(store.get().finds[0]!.run.state).toBe("done");
  expect(store.get().prs.map((one) => one.key)).toEqual(["Mele-Labs/pocket#7"]);
});

test("a search runs again only when he asks, a late start does not take a finished one back, and removing it drops it at Fleet", async () => {
  const { source, calls } = fake();
  const store = createReviewStore(source);
  store.find("Billing PRs");
  await settle();
  calls.emit(matched("Billing PRs"));
  calls.emit(search("Billing PRs", { state: "running" }));
  expect(store.get().finds[0]!.run.state).toBe("done");
  store.rerunFind("search:billing prs");
  await settle();
  expect(calls.searches).toEqual([{ words: "Billing PRs" }, { words: "Billing PRs", again: true }]);
  calls.emit(matched("Billing PRs", "done"));
  store.removeFind("search:billing prs");
  await settle();
  expect(calls.forgotten).toEqual(["Billing PRs"]);
  expect(store.get().finds).toEqual([]);
  expect(store.get().tab).toBe("requested");
  // An event that arrives after the removal does not bring it back.
  calls.emit(matched("Billing PRs", "done"));
  expect(store.get().finds).toEqual([]);
});

test("a search that failed is Fleet's reason, and one Fleet would not drop stays with the words it gave", async () => {
  const { source, calls } = fake({ forgetSearch: async () => said("The search is still going") });
  const store = createReviewStore(source);
  store.find("Billing PRs");
  await settle();
  calls.emit(search("Billing PRs", { state: "failed", failed: "The model answered something that was not matches" }));
  expect(store.get().finds[0]!.run).toMatchObject({ state: "failed", said: "The model answered something that was not matches" });
  store.removeFind("search:billing prs");
  await settle();
  expect(store.get().finds).toHaveLength(1);
  expect(store.get().failed).toBe("The search is still going");
});

const WALK = (number: number, over: Partial<ReviewRun> = {}): ReviewRun =>
  run({ number, name: "walk", state: "done", result: { shape: "web_walk", place: { how: "deployed", address: "https://pr-1.preview.test" }, steps: [{ say: "The list is there", look: { role: "region", name: "Jobs" } }] }, ...over });

const found = (over: Partial<WalkPlaceFound> = {}): WalkPlaceFound => ({ area: "crates/fleet", tried: [{ source: "pull_request", found: false, said: "No preview link in the comments or the checks" }], ...over });

test("the Walk writes against the place Fleet found, and a press alone starts no run", async () => {
  const place = { how: "deployed", address: "https://pr-1.preview.test" } as const;
  const { store, calls, key } = await opened({ findPlace: async () => ({ ok: true, value: found({ place, tried: [{ source: "pull_request", found: true, said: "Preview ready", by: "preview-bot" }] }) }) });
  const walk = [...(store.get().tools["m1"] ?? []), COMMENTS_TOOL].find((one) => one.mark === "walk") ?? { id: "walk", kind: "tool", name: "Walk", source: "armada", mark: "walk", checks: "", agent: true };
  store.runTool(key, walk as ToolDef);
  await settle();
  expect(calls.starts).toEqual([]);
  expect(store.get().work[key]!.view).toBe("walk");
  store.findPlace(key);
  await settle();
  expect(store.get().work[key]!.walk.find).toMatchObject({ state: "found" });
  expect(calls.starts).toEqual([{ number: 1, kind: "tool", name: "walk", place }]);
  expect(store.get().sheet).toBeUndefined();
});

test("a place nobody knows is asked, the answer is remembered for the area, and the walk is written against it", async () => {
  const { store, calls, key } = await opened({ findPlace: async () => ({ ok: true, value: found() }) });
  store.findPlace(key);
  await settle();
  expect(store.get().work[key]!.walk).toMatchObject({ asking: true });
  expect(store.get().sheet).toEqual({ kind: "ask", key });
  expect(calls.starts).toEqual([]);
  // Esc leaves the question open on the page, and it can be put up again.
  store.closeSheet();
  expect(store.get().work[key]!.walk.asking).toBe(true);
  store.askWhere(key);
  expect(store.get().sheet).toEqual({ kind: "ask", key });
  const place = { how: "pull", command: "armada fleet --dev", api: "http://localhost:7420" } as const;
  store.answer(key, place, "crates/fleet");
  await settle();
  expect(calls.saved).toEqual([{ manifestId: "m1", area: "crates/fleet" }]);
  expect(calls.starts).toEqual([{ number: 1, kind: "tool", name: "walk", place }]);
  expect(store.get().work[key]!.walk).toMatchObject({ asking: false, find: { state: "found", found: { place } } });
  expect(store.get().sheet).toBeUndefined();
});

test("a place answered and not remembered is not saved", async () => {
  const { store, calls, key } = await opened({ findPlace: async () => ({ ok: true, value: found() }) });
  store.findPlace(key);
  await settle();
  store.answer(key, { how: "deployed", address: "http://localhost:5173" }, undefined);
  await settle();
  expect(calls.saved).toEqual([]);
  expect(calls.starts).toHaveLength(1);
});

test("Walk it holds the place up, then opens the walk window once it is ready, and closing the window clears it", async () => {
  const { store, calls, key } = await opened({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z", { holder: { id: "s1", title: "Fix the wait" } })] }) }) });
  calls.emit(WALK(1, { result: { shape: "web_walk", place: { how: "pull", command: "armada fleet --dev", address: "http://localhost:7420" }, steps: [{ say: "The list is there", look: { role: "region", name: "Jobs" } }] } }));
  store.walkIt(key);
  expect(store.get().work[key]!.walk.opening).toBe(true);
  await settle();
  expect(calls.opened).toEqual([{ number: 1, how: "pull" }]);
  expect(calls.shown).toEqual([]);
  const walk: ReviewWalk = { id: "w1", manifest_id: "m1", number: 1, state: "starting", slot: 3, address: "http://localhost:7420" };
  calls.walk(walk);
  expect(store.get().work[key]!.walk.held).toMatchObject({ state: "starting", slot: 3 });
  calls.walk({ ...walk, state: "ready" });
  await settle();
  expect(calls.shown).toEqual([
    {
      walkId: "w1",
      manifestId: "m1",
      number: 1,
      title: "#1 Pull request 1",
      address: "http://localhost:7420",
      steps: [{ say: "The list is there", look: { role: "region", name: "Jobs" } }],
      holder: { id: "s1", title: "Fix the wait" },
    },
  ]);
  expect(store.get().work[key]!.walk.opening).toBe(false);
  // Ready twice opens one window.
  calls.walk({ ...walk, state: "ready" });
  await settle();
  expect(calls.shown).toHaveLength(1);
  calls.closed("w1");
  expect(store.get().work[key]!.walk.held).toBeUndefined();
});

test("an API walk is shown as its requests, and a walk that fails to start says which line broke, in Fleet's words", async () => {
  const requests = [{ say: "Read the plan windows", method: "GET", path: "/plan-usage", look: "read_at is there" }] as const;
  const { store, calls, key } = await opened();
  calls.emit(run({ name: "walk", result: { shape: "api_walk", place: { how: "deployed", api: "http://localhost:7420" }, requests: [...requests] } }));
  store.walkIt(key);
  await settle();
  calls.walk({ id: "w1", manifest_id: "m1", number: 1, state: "ready", address: "http://localhost:7420" });
  await settle();
  expect(calls.shown[0]).toMatchObject({ requests, address: "http://localhost:7420" });
  expect(calls.shown[0]!.steps).toBeUndefined();

  const broken = await opened({ openWalk: async () => ({ ok: true, value: { id: "w2", manifest_id: "m1", number: 1, state: "checking_out", address: "" } }) });
  broken.calls.emit(WALK(1, { result: { shape: "web_walk", place: { how: "pull", command: "pnpm dev", address: "http://localhost:5173" }, steps: [{ say: "x", look: { role: "main" } }] } }));
  broken.store.walkIt(broken.key);
  await settle();
  broken.calls.walk({ id: "w2", manifest_id: "m1", number: 1, state: "starting", slot: 2, address: "http://localhost:5173" });
  broken.calls.walk({ id: "w2", manifest_id: "m1", number: 1, state: "failed", address: "http://localhost:5173", failed: "pnpm dev exited with 1" });
  const walk = broken.store.get().work[broken.key]!.walk;
  expect(walk).toMatchObject({ reached: "starting", said: "pnpm dev exited with 1", opening: false, held: undefined });
  expect(broken.calls.shown).toEqual([]);
});

test("a note taken in the walk window where no Session holds the pull request stays here", async () => {
  const { store, calls, key } = await opened();
  calls.note({ manifestId: "m1", number: 1, at: "2026-10-16T10:05:00Z", text: "The list flickers", location: "region Jobs" });
  expect(store.get().work[key]!.walk.notes).toEqual([{ at: "2026-10-16T10:05:00Z", text: "The list flickers", location: "region Jobs" }]);
});

test("the places Armada remembers are read when the sheet opens, and one can be changed or forgotten", async () => {
  const kept: RememberedPlace = { id: "p1", manifest_id: "m1", repository: "Mele-Labs/armada", area: "crates/fleet", place: { how: "pull", command: "armada fleet --dev", api: "http://localhost:7420" } };
  const { store, calls } = await opened({
    places: async () => ({ ok: true, value: { places: [kept] } }),
    forgetPlace: async () => ({ ok: true, value: { places: [] } }),
  });
  store.showPlaces();
  expect(store.get().sheet).toEqual({ kind: "places" });
  await settle();
  expect(store.get().places).toMatchObject({ state: "read", list: [kept] });
  store.editPlace(kept, { how: "pull", command: "armada fleet --dev --port 7421", api: "http://localhost:7421" });
  await settle();
  expect(calls.saved).toEqual([{ manifestId: "m1", area: "crates/fleet" }]);
  expect(store.get().places.list[0]!.place).toMatchObject({ command: "armada fleet --dev --port 7421" });
  store.forgetPlace(kept);
  await settle();
  expect(store.get().places.list).toEqual([]);
});

test("a place Fleet would not read says why, and a place it would not keep leaves the walk running", async () => {
  const { store, calls, key } = await opened({ findPlace: async () => ({ ok: true, value: found() }), savePlace: async () => said("That area is not a directory") });
  store.findPlace(key);
  await settle();
  store.answer(key, { how: "deployed", address: "http://localhost:5173" }, "not a dir");
  await settle();
  expect(calls.starts).toHaveLength(1);
  expect(store.get().work[key]!.walk.said).toBe("That area is not a directory");

  const refused = await opened({ findPlace: async () => said("Fleet does not serve this repository") });
  refused.store.findPlace(refused.key);
  await settle();
  expect(refused.store.get().work[refused.key]!.walk.find).toEqual({ state: "said", said: "Fleet does not serve this repository" });
});
