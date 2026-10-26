// The Issues source over a fake of the calls Fleet answers: a tracker that cannot be read is Fleet's words, issues and queries
// are drawn in the tab's shapes, a query is run with `search_issues` and never `write_issue_query`, a new saved query goes without
// an id so Fleet gives it one, and a refusal carries the tracker's own words.

import type { IssueList, IssueQueries, IssueRead, SaveIssueQuery, TrackedIssue, WrittenIssueQuery } from "@armada/protocol";
import { expect, test } from "vitest";

import type { IssueQueryActed, IssuesApi } from "../../../shared/api/issues";
import { issuesSourceOf } from "./source";

const tracked = (reference: string, over: Partial<TrackedIssue> = {}): TrackedIssue => ({
  reference,
  title: `Issue ${reference}`,
  url: `https://example.test/${reference}`,
  labels: ["bug"],
  assignees: ["nick"],
  updated_at: "2026-10-20T12:00:00Z",
  excerpt: "Start of the body",
  linked_pull_request: false,
  state: "open",
  fields: [{ name: "Labels", values: ["bug"] }],
  holders: ["01JOB"],
  ...over,
});

const queries = (manifest: string, over: Partial<IssueQueries> = {}): IssueQueries => ({
  manifest_id: manifest,
  guide: { language: "GitHub search", open: "is:open", assigned_to_me: "is:open assignee:@me", fields: ["Labels", "Milestone"] },
  saved: [{ id: "q1", name: "Bugs", words: "bugs", query: "label:bug", row_fields: ["Labels"] }],
  ...over,
});

/** A refusal as Fleet words it: a 422 carrying the tracker's own sentence. */
const refused = (message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.issue_query_refused", message } } }) as never;

type Sent = { searches: [string, string][]; writes: [string, string][]; saves: [string, SaveIssueQuery][]; forgets: [string, string][] };

function api(over: Partial<IssuesApi> = {}) {
  const sent: Sent = { searches: [], writes: [], saves: [], forgets: [] };
  const ok = <T>(value: T): IssueQueryActed<T> => ({ ok: true, value });
  const read: IssueRead = {
    reference: "#1",
    title: "Coupon field",
    url: "https://example.test/1",
    state: "open",
    body: "The whole body",
    author: "dana",
    opened_at: "2026-10-01T09:00:00Z",
    labels: [],
    assignees: [],
    comments: [],
    linked_pull_requests: [{ number: 9, title: "Fix it", url: "https://example.test/pull/9", state: "open" }],
    fields: [{ name: "Milestone", values: ["v1"] }],
    holders: ["01JOB"],
  };
  const fake = {
    listIssueQueries: async (manifest: string) => ok(queries(manifest)),
    searchIssues: async (manifest: string, query: string) => (sent.searches.push([manifest, query]), ok<IssueList>({ manifest_id: manifest, fetched_at: "now", issues: [tracked("#1"), tracked("#2", { author: "dana" })] })),
    getIssue: async () => ok(read),
    writeIssueQuery: async (manifest: string, words: string) => (sent.writes.push([manifest, words]), ok<WrittenIssueQuery>({ query: "is:open label:bug", row_fields: ["Labels"], issues: [tracked("#3")], cost_micros: 1200 })),
    saveIssueQuery: async (manifest: string, save: SaveIssueQuery) => (sent.saves.push([manifest, save]), ok({ id: save.id ?? "new", name: save.name, words: save.words, query: save.query, row_fields: save.row_fields })),
    forgetIssueQuery: async (manifest: string, id: string) => (sent.forgets.push([manifest, id]), ok(queries(manifest, { saved: [{ id: "q2", name: "Left", words: "", query: "label:left", row_fields: [] }] }))),
    ...over,
  } as unknown as IssuesApi;
  return { source: issuesSourceOf(fake), sent };
}

test("a tracker is its guide's two queries and the queries saved, and one that cannot be read is Fleet's reason", async () => {
  const { source } = api();
  const withoutGuide = api({
    listIssueQueries: async (manifest: string) => {
      const { guide: _guide, ...bare } = queries(manifest, { refused: "no tracker is called `linear`" });
      return { ok: true, value: bare };
    },
  } as Partial<IssuesApi>);
  expect(await source.trackers(["storefront"])).toEqual({
    ok: true,
    value: [{ manifestId: "storefront", state: "read", name: "GitHub search", open: "is:open", assigned: "is:open assignee:@me", saved: [{ id: "q1", name: "Bugs", words: "bugs", query: "label:bug", row_fields: ["Labels"] }] }],
  });
  expect(await withoutGuide.source.trackers(["mobile"])).toEqual({ ok: true, value: [{ manifestId: "mobile", state: "refused", said: "no tracker is called `linear`" }] });
});

test("a repository Fleet does not serve is left out, and Fleet not answering at all is said", async () => {
  const served = api({ listIssueQueries: async (manifest: string) => (manifest === "gone" ? refused("repository gone is not served") : { ok: true, value: queries(manifest) }) } as Partial<IssuesApi>);
  const some = await served.source.trackers(["storefront", "gone"]);
  expect(some.ok && some.value.map((one) => one.manifestId)).toEqual(["storefront"]);
  const down = api({ listIssueQueries: async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) } as Partial<IssuesApi>);
  const none = await down.source.trackers(["storefront"]);
  expect(none.ok).toBe(false);
});

test("a query is run with search_issues and its issues are drawn with their holders", async () => {
  const { source, sent } = api();
  const answer = await source.run("storefront", "is:open label:bug");
  expect(sent.searches).toEqual([["storefront", "is:open label:bug"]]);
  expect(sent.writes).toEqual([]);
  expect(answer.ok && answer.value.map((one) => [one.reference, one.author, one.holders])).toEqual([
    ["#1", undefined, ["01JOB"]],
    ["#2", "dana", ["01JOB"]],
  ]);
});

test("a query the tracker refuses is the tracker's words", async () => {
  const { source } = api({ searchIssues: async () => refused("Field 'colour' does not exist") } as Partial<IssuesApi>);
  expect(await source.run("storefront", "colour:red")).toEqual({ ok: false, said: "Field 'colour' does not exist" });
});

/** A refusal whose fields name the program the tracker could not run. */
const lacking = (tool: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.issue_query_refused", message: "searching the issues did not happen: bd is not installed", fields: { missing_tool: tool } } } }) as never;

test("a search that failed for a missing program says which, beside the tracker's words", async () => {
  const { source } = api({ searchIssues: async () => lacking("bd") } as Partial<IssuesApi>);
  expect(await source.run("storefront", "status=open")).toEqual({ ok: false, said: "searching the issues did not happen: bd is not installed", missing: "bd" });
});

test("a refusal that names no tool carries none", async () => {
  const { source } = api({ searchIssues: async () => refused("Field 'colour' does not exist") } as Partial<IssuesApi>);
  expect(await source.run("storefront", "colour:red")).not.toHaveProperty("missing");
});

test("an install is asked of Fleet by tool, and a 200 that did not install is its said", async () => {
  const asked: [string, string][] = [];
  const done = api({ installIssueTool: async (manifest: string, tool: string) => (asked.push([manifest, tool]), { ok: true, value: { tool, done: true, said: "" } }) } as Partial<IssuesApi>);
  expect(await done.source.install("storefront", "bd")).toEqual({ ok: true, value: undefined });
  expect(asked).toEqual([["storefront", "bd"]]);
  const failed = api({ installIssueTool: async (_manifest: string, tool: string) => ({ ok: true, value: { tool, done: false, said: "Homebrew is not available" } }) } as Partial<IssuesApi>);
  expect(await failed.source.install("storefront", "bd")).toEqual({ ok: false, said: "Homebrew is not available" });
});

test("an issue read whole carries its body, its holders and the pull requests naming it", async () => {
  const { source } = api();
  const answer = await source.read("storefront", "#1");
  expect(answer.ok && answer.value).toMatchObject({
    reference: "#1",
    body: "The whole body",
    author: "dana",
    holders: ["01JOB"],
    fields: [{ name: "Milestone", values: ["v1"] }],
    linked_pull_requests: [{ number: 9, title: "Fix it", address: "https://example.test/pull/9", state: "open" }],
  });
});

test("a query written from words is the model's one call, and the issues it already ran come with it", async () => {
  const { source, sent } = api();
  const answer = await source.write("storefront", "open bugs");
  expect(sent.writes).toEqual([["storefront", "open bugs"]]);
  expect(answer.ok && answer.value.query).toBe("is:open label:bug");
  expect(answer.ok && answer.value.tickets?.map((one) => one.reference)).toEqual(["#3"]);
  expect(sent.searches).toEqual([]);
});

test("a new saved query is sent without an id, an edited one with it, and what is kept is read back", async () => {
  const { source, sent } = api();
  const kept = await source.save("storefront", { name: "Open bugs", words: "open bugs", query: "is:open label:bug", row_fields: ["Labels"] });
  expect(sent.saves[0]).toEqual(["storefront", { name: "Open bugs", words: "open bugs", query: "is:open label:bug", row_fields: ["Labels"] }]);
  expect(kept.ok && kept.value.map((one) => one.id)).toEqual(["q1"]);
  await source.save("storefront", { id: "q1", name: "Bugs", words: "bugs", query: "label:bug is:open", row_fields: ["Labels"] });
  expect(sent.saves[1]![1].id).toBe("q1");
});

test("a save the tracker refuses is its words and nothing is read back", async () => {
  const { source } = api({ saveIssueQuery: async () => refused("The query is not valid: unbalanced quote") } as Partial<IssuesApi>);
  expect(await source.save("storefront", { name: "x", words: "", query: '"', row_fields: [] })).toEqual({ ok: false, said: "The query is not valid: unbalanced quote" });
});

test("forgetting a saved query answers what is left", async () => {
  const { source, sent } = api();
  const answer = await source.remove("storefront", "q1");
  expect(sent.forgets).toEqual([["storefront", "q1"]]);
  expect(answer).toEqual({ ok: true, value: [{ id: "q2", name: "Left", words: "", query: "label:left", row_fields: [] }] });
});
