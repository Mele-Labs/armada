// The review store over a fake source: the lists merged by pull request, a list the forge would not answer keeping what
// it held, a page read when a pull request is opened, a patch read when its change is shown, and a review posted as one
// call with every comment placed by path, line and side. Nothing here touches `window.armada`.

import type { PullRequestPage, PullReviewLists, PullReviewRow, WritePullRequest } from "@armada/protocol";
import { expect, test } from "vitest";

import type { Read, ReviewSource } from "./source";
import { createReviewStore } from "./store";
import { NO_FIXTURES } from "./tools";

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

type Calls = { writes: WritePullRequest[]; resolves: { thread: string; resolved: boolean }[]; files: string[] };

function fake(over: Partial<ReviewSource> = {}): { source: ReviewSource; calls: Calls } {
  const calls: Calls = { writes: [], resolves: [], files: [] };
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
    ...over,
  };
  return { source, calls };
}

const settle = () => new Promise((done) => setTimeout(done, 0));

async function opened(over: Partial<ReviewSource> = {}) {
  const { source, calls } = fake({ lists: async () => ({ ok: true, value: lists({ requested: [row(1, "2026-10-16T10:00:00Z")] }) }), ...over });
  const store = createReviewStore(source, NO_FIXTURES);
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
  const store = createReviewStore(source, NO_FIXTURES);
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
  const store = createReviewStore(source, NO_FIXTURES);
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
  const store = createReviewStore(source, NO_FIXTURES);
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

test("where nothing runs behind the agent half, an agent tool answers nothing and Comments stays live", async () => {
  const { store, key } = await opened();
  expect(store.get().agentsBuilt).toBe(false);
  const tools = store.get().armada;
  const risks = tools.find((one) => one.id === "risks")!;
  const comments = tools.find((one) => one.id === "comments")!;
  store.runTool(key, risks);
  store.rerun(key, risks);
  store.explain(key, tools.find((one) => one.id === "how-it-works")!);
  store.ask(key, "Does this change anything?");
  store.find("PRs that touch files I have touched");
  const work = store.get().work[key]!;
  expect(work.runs).toEqual({});
  expect(work.findings).toEqual([]);
  expect(work.explains).toEqual({});
  expect(work.asks).toEqual([]);
  expect(store.get().finds).toEqual([]);
  store.runTool(key, comments);
  expect(store.get().work[key]!.view).toBe("comments");
});
