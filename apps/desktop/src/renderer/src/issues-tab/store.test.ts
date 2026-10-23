// The Issues store over a fake source: each repository's tracker read once, a query run when a list needs it and not again,
// the issue read when it is opened, a query written from his words only after he has said Run, and what is saved, edited and
// forgotten. A refusal is Fleet's words and not a thrown error. Nothing here touches `window.armada`.

import { expect, test } from "vitest";

import type { IssuesSource, QueryToSave, Read, SavedQuery, Ticket, TicketRead, Tracker, WrittenQuery } from "./model";
import { createIssuesStore } from "./store";

const ticket = (reference: string, over: Partial<Ticket> = {}): Ticket => ({
  reference,
  title: `Issue ${reference}`,
  url: `https://example.test/${reference}`,
  state: "open",
  assignees: [],
  updated_at: "2026-10-20T12:00:00Z",
  excerpt: "",
  fields: [],
  linked_pull_request: false,
  holders: [],
  ...over,
});

const tracker = (manifestId: string, saved: SavedQuery[] = []): Tracker => ({ manifestId, state: "read", name: "GitHub", open: "is:open", assigned: "is:open assignee:@me", saved });
const saved = (id: string, name = id): SavedQuery => ({ id, name, words: "", query: `label:${id}`, row_fields: ["Labels"] });
const said = (words: string): Read<never> => ({ ok: false, said: words });

type Calls = { trackers: (readonly string[])[]; runs: [string, string][]; writes: [string, string][]; saves: [string, QueryToSave][]; removes: [string, string][] };

function source(over: Partial<IssuesSource> = {}) {
  const calls: Calls = { trackers: [], runs: [], writes: [], saves: [], removes: [] };
  const kept: SavedQuery[] = [];
  const fake: IssuesSource = {
    trackers: async (ids) => (calls.trackers.push(ids), { ok: true, value: ids.map((id) => tracker(id)) }),
    run: async (manifestId, query) => (calls.runs.push([manifestId, query]), { ok: true, value: [ticket("#1"), ticket("#2")] }),
    read: async (_manifestId, reference) => ({ ok: true, value: { ...ticket(reference), body: "The body", linked_pull_requests: [] } satisfies TicketRead }),
    write: async (manifestId, words) => (calls.writes.push([manifestId, words]), { ok: true, value: { query: "is:open label:bug", row_fields: ["Labels"] } satisfies WrittenQuery }),
    save: async (manifestId, query) => {
      calls.saves.push([manifestId, query]);
      const one = { ...query, id: query.id ?? `new-${kept.length}` };
      const at = kept.findIndex((it) => it.id === one.id);
      if (at < 0) kept.push(one);
      else kept[at] = one;
      return { ok: true, value: [...kept] };
    },
    remove: async (manifestId, id) => {
      calls.removes.push([manifestId, id]);
      const at = kept.findIndex((it) => it.id === id);
      if (at >= 0) kept.splice(at, 1);
      return { ok: true, value: [...kept] };
    },
    ...over,
  };
  return { fake, calls, kept };
}

const settled = () => new Promise((resume) => setTimeout(resume, 0));

test("a repository's tracker is read once, and one that joins the scope is read alone", async () => {
  const { fake, calls } = source();
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  store.readTrackers(["storefront"]);
  await settled();
  expect(calls.trackers).toEqual([["storefront"]]);
  store.readTrackers(["storefront", "ops"]);
  await settled();
  expect(calls.trackers).toEqual([["storefront"], ["ops"]]);
  expect(store.get().trackers?.map((one) => one.manifestId)).toEqual(["storefront", "ops"]);
});

test("a query is run when a list first needs it, and the same query is not run again", async () => {
  const { fake, calls } = source();
  const store = createIssuesStore(fake);
  store.run("storefront", "is:open");
  expect(store.runOf("storefront", "is:open")).toEqual({ state: "running" });
  await settled();
  store.run("storefront", "is:open");
  expect(calls.runs).toEqual([["storefront", "is:open"]]);
  const run = store.runOf("storefront", "is:open");
  expect(run?.state === "read" && run.tickets.map((one) => one.reference)).toEqual(["#1", "#2"]);
  // The same words in another repository are another list.
  expect(store.runOf("ops", "is:open")).toBeUndefined();
});

test("a search the tracker refuses is its words, and running it again asks again", async () => {
  let answers: Read<Ticket[]>[] = [said("the tracker has no field `colour`")];
  const { fake, calls } = source({ run: async (manifestId, query) => (calls.runs.push([manifestId, query]), answers.shift() ?? { ok: true, value: [ticket("#9")] }) });
  const store = createIssuesStore(fake);
  store.run("storefront", "colour:red");
  await settled();
  expect(store.runOf("storefront", "colour:red")).toEqual({ state: "failed", said: "the tracker has no field `colour`" });
  store.run("storefront", "colour:red", true);
  await settled();
  expect(store.runOf("storefront", "colour:red")?.state).toBe("read");
  expect(calls.runs).toHaveLength(2);
});

test("an issue is read whole when it is opened, once, and a refusal is its words", async () => {
  const reads: string[] = [];
  const { fake } = source({
    read: async (_manifestId, reference) => (reads.push(reference), reference === "#404" ? said("#404 is not an issue of this repository") : { ok: true, value: { ...ticket(reference), body: "Whole", linked_pull_requests: [] } }),
  });
  const store = createIssuesStore(fake);
  store.open("storefront", "#7");
  await settled();
  store.open(undefined);
  store.open("storefront", "#7");
  await settled();
  expect(store.get().open).toEqual({ manifestId: "storefront", reference: "#7" });
  const read = store.readOf("storefront", "#7");
  expect(read?.state === "read" && read.ticket.body).toBe("Whole");
  expect(reads).toEqual(["#7"]);
  store.open("storefront", "#404");
  await settled();
  expect(store.readOf("storefront", "#404")).toEqual({ state: "failed", said: "#404 is not an issue of this repository" });
});

test("writing a query spends, so nothing is written until he says Run, and Cancel writes nothing", async () => {
  const { fake, calls } = source();
  const store = createIssuesStore(fake);
  store.spend("Describe a query", () => store.write("bugs nobody has", ["storefront"]));
  await settled();
  expect(store.get().spending).toBe("Describe a query");
  expect(calls.writes).toEqual([]);
  store.cancelSpend();
  await settled();
  expect(store.get().spending).toBeUndefined();
  expect(calls.writes).toEqual([]);

  store.spend("Describe a query", () => store.write("bugs nobody has", ["storefront"]));
  store.confirmSpend(false);
  await settled();
  expect(calls.writes).toEqual([["storefront", "bugs nobody has"]]);
  // Asked again next time, since he did not say otherwise.
  store.spend("Describe a query", () => store.write("more", ["storefront"]));
  expect(store.get().spending).toBe("Describe a query");
  expect(calls.writes).toHaveLength(1);
  // Do not ask again: the next one goes straight out.
  store.confirmSpend(true);
  await settled();
  store.spend("Describe a query", () => store.write("again", ["storefront"]));
  await settled();
  expect(store.get().spending).toBeUndefined();
  expect(calls.writes.map((one) => one[1])).toEqual(["bugs nobody has", "more", "again"]);
});

test("a query written is run for each repository as it arrives, and a repository that refuses it says so", async () => {
  const { fake, calls } = source({ write: async (manifestId) => (manifestId === "ops" ? said("the model answered nothing usable") : { ok: true, value: { query: "is:open label:bug", row_fields: ["Labels"] } }) });
  const store = createIssuesStore(fake);
  store.write("open bugs", ["storefront", "ops"]);
  await settled();
  const describing = store.get().describing;
  expect(describing?.phase === "written" && describing.written["ops"]).toEqual({ state: "failed", said: "the model answered nothing usable" });
  expect(describing?.phase === "written" && describing.written["storefront"]?.state).toBe("read");
  expect(calls.runs).toEqual([["storefront", "is:open label:bug"]]);
});

test("a query Fleet already ran for the words is the list, and is not run again", async () => {
  const { fake, calls } = source({ write: async () => ({ ok: true, value: { query: "is:open label:bug", row_fields: ["Labels"], tickets: [ticket("#5")] } }) });
  const store = createIssuesStore(fake);
  store.write("open bugs", ["storefront"]);
  await settled();
  const run = store.runOf("storefront", "is:open label:bug");
  expect(run?.state === "read" && run.tickets.map((one) => one.reference)).toEqual(["#5"]);
  expect(calls.runs).toEqual([]);
});

test("saving what was written keeps it in each repository under his name, with no id of the window's making", async () => {
  const { fake, calls } = source();
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  store.write("open bugs", ["storefront"]);
  await settled();
  await store.saveWritten("Open bugs");
  expect(calls.saves).toEqual([["storefront", { name: "Open bugs", words: "open bugs", query: "is:open label:bug", row_fields: ["Labels"] }]]);
  const one = store.get().trackers?.find((it) => it.manifestId === "storefront");
  expect(one?.state === "read" && one.saved.map((it) => it.name)).toEqual(["Open bugs"]);
  expect(store.get().describing).toBeUndefined();
});

test("a query edited by hand is kept under its id and run again as written, with no model", async () => {
  const { fake, calls, kept } = source();
  kept.push(saved("checkout"));
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  await settled();
  store.edit("storefront", "checkout");
  store.saveEdited("storefront", { ...saved("checkout"), query: "label:checkout is:open" });
  await settled();
  expect(calls.saves).toEqual([["storefront", { ...saved("checkout"), query: "label:checkout is:open" }]]);
  expect(calls.writes).toEqual([]);
  expect(calls.runs).toEqual([["storefront", "label:checkout is:open"]]);
  expect(store.get().editing).toBeUndefined();
  const one = store.get().trackers?.find((it) => it.manifestId === "storefront");
  expect(one?.state === "read" && one.saved.map((it) => it.query)).toEqual(["label:checkout is:open"]);
});

test("forgetting a saved query leaves what remains", async () => {
  const { fake, calls, kept } = source();
  kept.push(saved("a"), saved("b"));
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  await settled();
  store.remove("storefront", "a");
  await settled();
  expect(calls.removes).toEqual([["storefront", "a"]]);
  const one = store.get().trackers?.find((it) => it.manifestId === "storefront");
  expect(one?.state === "read" && one.saved.map((it) => it.id)).toEqual(["b"]);
});

test("a save or a forget Fleet refuses is its words, and keeps what the tab held", async () => {
  const { fake } = source({ save: async () => said("the tracker refuses `label:`"), remove: async () => said("no saved query has that id") });
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  await settled();
  store.saveEdited("storefront", saved("x"));
  await settled();
  expect(store.get().failed).toBe("the tracker refuses `label:`");
  store.remove("storefront", "x");
  await settled();
  expect(store.get().failed).toBe("no saved query has that id");
  const one = store.get().trackers?.find((it) => it.manifestId === "storefront");
  expect(one?.state === "read" && one.saved).toEqual([]);
});

test("Fleet not answering for the trackers is said, and asking again is allowed", async () => {
  let answers: Read<Tracker[]>[] = [said("Fleet is not running")];
  const { fake, calls } = source({ trackers: async (ids) => (calls.trackers.push(ids), answers.shift() ?? { ok: true, value: ids.map((id) => tracker(id)) }) });
  const store = createIssuesStore(fake);
  store.readTrackers(["storefront"]);
  await settled();
  expect(store.get().failed).toBe("Fleet is not running");
  expect(store.get().trackers).toBeUndefined();
  store.readTrackers(["storefront"]);
  await settled();
  expect(store.get().failed).toBeUndefined();
  expect(store.get().trackers?.map((one) => one.manifestId)).toEqual(["storefront"]);
});
