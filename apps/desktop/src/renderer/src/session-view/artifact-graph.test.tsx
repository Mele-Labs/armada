// The Artifacts graph as data: what each node hangs off, which joins it draws, and that it draws without a history. Browser, since the rows carry glyphs.

import { expect, test } from "vitest";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

import { artifactsOf, matching } from "./artifacts";
import { graphOf } from "./artifact-graph";

type Versions = NonNullable<Session["versions"]>;

const said = (id: string, text: string): SessionRow => ({ id, at: `2026-10-22T10:0${id.slice(1)}:00Z`, kind: "message", from: { kind: "you" }, text });
const note = (url: string, by: "you" | "agent" = "agent"): SessionAttachment => ({ kind: "link", url, by });
const issue = (state: "open" | "closed", pulls?: number[]): SessionAttachment => ({ kind: "issue", number: "5", title: "Retry the lease", address: "https://forge/issues/5", state, ...(pulls === undefined ? {} : { pull_requests: pulls }) });
const pull = (closes?: number[]): SessionAttachment => ({ kind: "pull_request", number: 7, title: "Retry", branch: "armada/retry", address: "https://forge/7", checks: { state: "passed" }, state: "open", auto: false, ...(closes === undefined ? {} : { closes }) });

const PLAN = {
  version: 1,
  scope: [],
  approved: true,
  after: "m1",
  groups: [
    { id: "g1", agent: "rust-engineer", title: "Retry it", paths: [], tasks: [{ id: "t1", text: "Take the lease again", state: "done" as const }] },
    { id: "g2", agent: "rust-engineer", title: "Test it", paths: [], tasks: [{ id: "t2", text: "Fake the clock", state: "done" as const }] },
  ],
};

const job = (attachments: SessionAttachment[], versions?: Versions, plan = true): Session =>
  ({
    id: "j1",
    title: "Retry the lease",
    attachments,
    rows: [said("m1", "Read the first note"), said("m2", "Then the second")],
    turn: { state: "idle" },
    ...(versions === undefined ? {} : { versions }),
    ...(plan ? { work: { plan: PLAN } } : {}),
  }) as unknown as Session;

const child = { id: "c1", title: "Fake the clock", attachments: [{ kind: "started_by", id: "j1", group: "g2" }], rows: [], turn: { state: "idle" }, standing: "idle" } as unknown as Session;

const acts = { sketch: () => undefined, clock: () => "10:00" };

function graph(session: Session, words = "") {
  const all = artifactsOf(session, [], { sessions: [child], open: () => undefined });
  return graphOf(session, all, matching(all, words), words, acts);
}
const edgeOf = (model: ReturnType<typeof graph>, target: string) => model.edges.filter((one) => one.target === target).map((one) => `${one.source} ${one.kind}`);
const at = (minute: number, attachment: SessionAttachment, from?: { message?: string; task?: string }) => ({ at: `2026-10-22T10:${String(minute).padStart(2, "0")}:00Z`, attachment, ...(from === undefined ? {} : { from }) });

test("an artifact that followed a task hangs off that task's group", () => {
  const link = note("https://example.com/a");
  const model = graph(job([link], [at(10, link, { task: "t2", message: "m1" })]));
  expect(edgeOf(model, "link:https://example.com/a")).toEqual(["group:g2 from"]);
});

test("an artifact that followed a message and no task hangs off that message", () => {
  const link = note("https://example.com/a", "you");
  const model = graph(job([link], [at(10, link, { message: "m2" })]));
  expect(edgeOf(model, "link:https://example.com/a")).toEqual(["message:m2 from"]);
});

test("an artifact that followed neither hangs off the first message", () => {
  const link = note("https://example.com/a");
  const model = graph(job([link], [at(10, link)]));
  expect(edgeOf(model, "link:https://example.com/a")).toEqual(["message:m1 from"]);
});

test("a later version that followed something else draws a second, thin edge, and one that followed the same draws none", () => {
  const link = note("https://example.com/a");
  const model = graph(job([link], [at(10, link, { task: "t1" }), at(20, link, { task: "t1" }), at(30, link, { message: "m2" })]));
  expect(edgeOf(model, "link:https://example.com/a")).toEqual(["group:g1 from", "message:m2 again"]);
});

test("an issue is joined to the pull request that closes it", () => {
  const model = graph(job([issue("open"), pull([5])], [at(10, issue("open")), at(20, pull([5]), { task: "t2" })]));
  expect(edgeOf(model, "pr:7")).toEqual(["group:g2 from", "issue:5 join"]);
});

test("an issue is joined to a pull request it names, when the pull request does not name it", () => {
  const model = graph(job([issue("open", [7]), pull()], [at(10, issue("open", [7])), at(20, pull(), { task: "t2" })]));
  expect(edgeOf(model, "pr:7")).toContain("issue:5 join");
});

test("a Job started from a group hangs off that group", () => {
  const started: SessionAttachment = { kind: "started_to", id: "c1", group: "g2" };
  const model = graph(job([started], [at(10, started, { task: "t1" })]));
  expect(edgeOf(model, "session:c1")).toEqual(["group:g2 from"]);
});

test("with no history every artifact is drawn once and hangs off the first message", () => {
  const model = graph(job([note("https://example.com/a"), pull()], undefined, false));
  expect(model.nodes.filter((one) => one.kind === "artifact").map((one) => [one.id, one.marks.length])).toEqual([
    ["link:https://example.com/a", 0],
    ["pr:7", 0],
  ]);
  expect(model.edges.map((one) => `${one.source}>${one.target}`)).toEqual(["message:m1>link:https://example.com/a", "message:m1>pr:7"]);
});

test("a version of something the ledger no longer holds is not drawn", () => {
  const gone = note("https://example.com/gone");
  const model = graph(job([], [at(10, gone, { task: "t1" })]));
  expect(model.nodes.filter((one) => one.kind === "artifact")).toEqual([]);
});

test("a search dims the nodes it does not match and drops none", () => {
  const model = graph(job([note("https://example.com/a"), pull()], undefined, false), "retry");
  const dim = Object.fromEntries(model.nodes.map((one) => [one.id, one.dim]));
  expect(dim["pr:7"]).toBe(false);
  expect(dim["link:https://example.com/a"]).toBe(true);
  expect(model.nodes.length).toBe(Object.keys(dim).length);
});
