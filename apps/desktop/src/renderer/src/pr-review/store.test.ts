// The review store over a fake source: the lists merged by pull request, a list the forge would not answer keeping what
// it held, a page read when a pull request is opened, a patch read when its change is shown, and a review posted as one
// call with every comment placed by path, line and side. The agent half is runs Fleet keeps: read when a pull request is opened,
// moved by `review_run.changed`, started only on a press, and shown failed in Fleet's words. Nothing here touches `window.armada`.

import type { PullRequestPage, PullReviewLists, PullReviewRow, ReviewPlace, ReviewRun, ReviewTools, StartReviewRun, WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";

import type { ToolDef } from "./model";
import type { Read, ReviewSource } from "./source";
import { NO_SEARCH, createReviewStore } from "./store";
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

type Calls = { writes: WritePullRequest[]; resolves: { thread: string; resolved: boolean }[]; files: string[]; starts: StartReviewRun[]; saves: string[]; emit: (run: ReviewRun) => void };

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
const place = (start: number, end = start, over: Partial<ReviewPlace> = {}): ReviewPlace => ({ path: "a.rs", side: "right", start, end, ...over });

function fake(over: Partial<ReviewSource> = {}): { source: ReviewSource; calls: Calls } {
  let told: (run: ReviewRun) => void = () => undefined;
  const calls: Calls = { writes: [], resolves: [], files: [], starts: [], saves: [], emit: (one) => told(one) };
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
    ...over,
  };
  return { source, calls };
}

const settle = () => new Promise((done) => setTimeout(done, 0));

async function opened(over: Partial<ReviewSource> = {}) {
  const { source, calls } = fake({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z")] }) }), ...over });
  const store = createReviewStore(source, NO_SEARCH);
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
  const store = createReviewStore(source, NO_SEARCH);
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
  const store = createReviewStore(source, NO_SEARCH);
  store.fetch();
  await settle();
  store.fetch();
  await settle();
  // Requested answered empty and lost its row. Assigned was silent and kept its own.
  expect(store.get().prs.map((one) => [one.number, one.lists])).toEqual([[2, ["assigned"]]]);
});

test("a refused read says why and leaves the lists as they were", async () => {
  let reads = 0;
  const { source } = fake({ lists: async () => (reads++ === 0 ? { ok: true, value: lists({ requested: [row(1, "2026-10-16T08:00:00Z")] }) } : said("gh is not signed in")) });
  const store = createReviewStore(source, NO_SEARCH);
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

test("Natural Search stays off where no mock forge holds what a find would come to", async () => {
  const { store } = await opened();
  expect(store.get().searchBuilt).toBe(false);
  store.find("PRs that touch files I have touched");
  expect(store.get().finds).toEqual([]);
});
