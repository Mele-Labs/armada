// What the store behind the window's Sessions does with what main publishes.

import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { SessionRecord } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import type { StateChange } from "../../shared/state-change";
import { QUEUED_KEY } from "./queued-ideas";
import { WiredStore } from "./sessions-wired";

const AT = "2026-10-07T13:48:02Z";

const record = (id: string): SessionRecord => ({
  id,
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
});

/** A store fed by hand: what `publish` is told is what the window is sent, as a new copy each time. */
function fed() {
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof send) => ((send = on), () => undefined) } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  return { store, publish: (change: Partial<BridgeState>) => send({ full: structuredClone({ ...NOTHING_YET, ...change }), seq: 0 }) };
}

it("hands back the same Sessions when a publish carries nothing new, and the same one for each that did not change", () => {
  const { store, publish } = fed();
  publish({ sessions: { state: "read", sessions: [record("a"), record("b")] } });
  const first = store.draft.get();
  publish({ sessions: { state: "read", sessions: [record("a"), record("b")] } });
  expect(store.draft.get()).toBe(first);

  publish({ sessions: { state: "read", sessions: [{ ...record("a"), title: "Named" }, record("b")] } });
  const next = store.draft.get();
  expect(next).not.toBe(first);
  expect(next[1]).toBe(first[1]);
  expect(next[0]).toMatchObject({ title: "Named" });
});

it("keeps the Sessions it holds when a later publish says the read failed", () => {
  const { store, publish } = fed();
  publish({ sessions: { state: "read", sessions: [record("a")] } });
  const first = store.draft.get();
  publish({ sessions: { state: "failed", outcome: { ok: false, why: "not_connected" } } });
  expect(store.draft.get()).toBe(first);
  expect(store.served()).toBe(true);
});

it("reads the open follow-ups off the record, and a Drop settles one as dropped by its id", () => {
  const settleLater = vi.fn(async () => ({ ok: true }));
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof send) => ((send = on), () => undefined), settleLater } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  const later = [{ id: "later-2", sentence: "The Studio icon tooltip is still owed.", said_at: AT }];
  send({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [{ ...record("a"), later }] } }), seq: 0 });
  expect(store.draft.get()[0]?.later).toEqual([{ id: "later-2", text: "The Studio icon tooltip is still owed.", at: AT }]);
  store.draft.dropLater!("a", "later-2");
  expect(settleLater).toHaveBeenCalledWith({ session_id: "a", id: "later-2", how: "dropped" });
});

// ---- Ideas saved to start later, and the model and effort a start carries ----------------------------------

/** A window with the one thing the queue reads: the storage every window of Bridge shares, and the event that says another wrote it. */
function windowWithStorage() {
  const kept = new Map<string, string>();
  const heard = new Set<(event: { key: string }) => void>();
  vi.stubGlobal("window", {
    localStorage: { getItem: (key: string) => kept.get(key) ?? null, setItem: (key: string, value: string) => void kept.set(key, value), removeItem: (key: string) => void kept.delete(key) },
    addEventListener: (type: string, on: (event: { key: string }) => void) => type === "storage" && heard.add(on),
    removeEventListener: (type: string, on: (event: { key: string }) => void) => type === "storage" && heard.delete(on),
  });
  return { kept, otherWindowWrote: () => heard.forEach((on) => on({ key: QUEUED_KEY })) };
}

const IDEA = { text: "Make the Cleanup grid sortable by size", root: "/code/armada", repository: "armada" };

/** A store whose Fleet answers a start with `answer`, and whose window already holds the Session it will name. */
async function starting(answer: "ok" | "refused") {
  const startSession = vi.fn(async () => (answer === "ok" ? { ok: true as const, value: record("new") } : { ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } }));
  const sendSessionMessage = vi.fn(async () => ({ ok: true as const }));
  let wire: (change: StateChange<BridgeState>) => void = () => undefined;
  const send = (state: BridgeState) => wire({ full: state, seq: 0 });
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof wire) => ((wire = on), () => undefined), startSession, sendSessionMessage } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  // Its first read of the state settles first, and what is published after it is what the window holds.
  await new Promise((done) => setTimeout(done, 0));
  send(structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [record("new")] } }));
  return { store, startSession, sendSessionMessage };
}

let storage: ReturnType<typeof windowWithStorage>;
beforeEach(() => {
  storage = windowWithStorage();
});
afterEach(() => vi.unstubAllGlobals());

it("keeps a queued idea with its words, repository, model and effort, oldest first, across a restart", () => {
  const first = new WiredStore({} as BridgeApi);
  first.draft.queue!(IDEA);
  first.draft.queue!({ ...IDEA, text: "Draft the billing export notes", root: "/code/billing", repository: "billing", model: "opus", effort: "high" });

  // A new window after a restart reads the same store.
  const restarted = new WiredStore({} as BridgeApi);
  const ideas = restarted.draft.queued!();
  expect(ideas.map((one) => one.text)).toEqual(["Make the Cleanup grid sortable by size", "Draft the billing export notes"]);
  expect(ideas[0]).toMatchObject({ root: "/code/armada", repository: "armada" });
  expect(ideas[0]).not.toHaveProperty("model");
  expect(ideas[1]).toMatchObject({ root: "/code/billing", repository: "billing", model: "opus", effort: "high" });
  expect(new Set(ideas.map((one) => one.id)).size).toBe(2);
});

it("hands back the same queue until it changes, and tells readers when it does", () => {
  const store = new WiredStore({} as BridgeApi);
  const told = vi.fn();
  store.draft.subscribe(told);
  const empty = store.draft.queued!();
  expect(store.draft.queued!()).toBe(empty);
  store.draft.queue!(IDEA);
  expect(told).toHaveBeenCalled();
  expect(store.draft.queued!()).not.toBe(empty);
});

it("drops an idea without starting anything", async () => {
  const { store, startSession } = await starting("ok");
  store.draft.queue!(IDEA);
  store.draft.dropQueued!(store.draft.queued!()[0]!.id);
  expect(store.draft.queued!()).toEqual([]);
  expect(JSON.parse(storage.kept.get(QUEUED_KEY)!)).toEqual([]);
  expect(startSession).not.toHaveBeenCalled();
});

it("starts a queued idea through the real start, with its words as the request and its picks as the tuning, and takes it off the queue", async () => {
  const { store, startSession, sendSessionMessage } = await starting("ok");
  store.draft.queue!({ ...IDEA, model: "opus", effort: "high" });
  store.draft.queue!({ ...IDEA, text: "Another" });
  const [first] = store.draft.queued!();

  const id = await store.draft.startQueued!(first!.id);

  expect(id).toBe("new");
  expect(startSession).toHaveBeenCalledWith(undefined, "/code/armada", IDEA.text, { model: "opus", effort: "high" });
  expect(store.draft.queued!().map((one) => one.text)).toEqual(["Another"]);
  await vi.waitFor(() => expect(sendSessionMessage).toHaveBeenCalledWith(expect.objectContaining({ session_id: "new", text: IDEA.text })));
});

it("leaves a pick out of the start where the idea has none, so the proposer fills it in", async () => {
  const { store, startSession } = await starting("ok");
  store.draft.queue!(IDEA);
  await store.draft.startQueued!(store.draft.queued!()[0]!.id);
  expect(startSession).toHaveBeenCalledWith(undefined, "/code/armada", IDEA.text, {});
});

/** A store whose Fleet answers Auto land as `answer` says, with the window's refusals caught in `told`. */
async function landing(answer: "ok" | "refused" | "forge_refused") {
  const error = { code: answer === "forge_refused" ? "fleet.auto_merge_refused" : "fleet.session_command_refused", message: "Auto merge is not allowed on this repository" };
  const setSessionAutoLand = vi.fn(async () => (answer === "ok" ? { ok: true as const, value: record("a") } : { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } }));
  const told = vi.fn();
  vi.stubGlobal("window", { ...(window as object), dispatchEvent: told });
  vi.stubGlobal("CustomEvent", class { constructor(readonly type: string, readonly init: { detail: unknown }) {} get detail() { return this.init.detail; } });
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: () => () => undefined, setSessionAutoLand } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  await new Promise((done) => setTimeout(done, 0));
  return { store, setSessionAutoLand, said: () => told.mock.calls.map(([event]) => (event as { detail: { sentence: string } }).detail.sentence) };
}

it("turns Auto land on through Fleet and says nothing", async () => {
  const { store, setSessionAutoLand, said } = await landing("ok");
  expect(await store.draft.autoLand!("a", true)).toBe(true);
  expect(setSessionAutoLand).toHaveBeenCalledWith("a", true);
  expect(said()).toEqual([]);
});

it("says the forge's refusal of auto-merge and still counts the flag as changed, since Fleet kept it first", async () => {
  const { store, said } = await landing("forge_refused");
  expect(await store.draft.autoLand!("a", true)).toBe(true);
  expect(said()).toEqual(["Auto merge is not allowed on this repository"]);
});

it("says a refusal that changed nothing, and counts the flag as unchanged", async () => {
  const { store, said } = await landing("refused");
  expect(await store.draft.autoLand!("a", true)).toBe(false);
  expect(said()).toEqual(["Auto merge is not allowed on this repository"]);
});

it("keeps the idea when Fleet refuses the start", async () => {
  const { store, startSession, sendSessionMessage } = await starting("refused");
  store.draft.queue!(IDEA);
  const [idea] = store.draft.queued!();

  expect(await store.draft.startQueued!(idea!.id)).toBeUndefined();

  expect(startSession).toHaveBeenCalledTimes(1);
  expect(store.draft.queued!()).toEqual([idea]);
  expect(sendSessionMessage).not.toHaveBeenCalled();
});

it("starts an idea once when it is pressed twice", async () => {
  const { store, startSession } = await starting("ok");
  store.draft.queue!(IDEA);
  const [idea] = store.draft.queued!();
  const [one, two] = await Promise.all([store.draft.startQueued!(idea!.id), store.draft.startQueued!(idea!.id)]);
  expect([one, two].filter((id) => id !== undefined)).toEqual(["new"]);
  expect(startSession).toHaveBeenCalledTimes(1);
});

it("passes the model and effort of a dispatcher start to Fleet, and none where none was picked", async () => {
  const { store, startSession } = await starting("ok");
  await store.draft.start(undefined, "/code/armada", "fix it", { model: "sonnet" });
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "fix it", { model: "sonnet" });
  await store.draft.start(undefined, "/code/armada", "fix it");
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "fix it", undefined);
});

it("shows in this window what another window queued, and what it started", () => {
  const store = new WiredStore({ syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: () => () => undefined } as unknown as BridgeApi);
  store.attach();
  const told = vi.fn();
  store.draft.subscribe(told);
  expect(store.draft.queued!()).toEqual([]);

  // The other window's write lands in the shared store, and the browser tells this one.
  storage.kept.set(QUEUED_KEY, JSON.stringify([{ id: "q-other", ...IDEA }]));
  storage.otherWindowWrote();

  expect(store.draft.queued!().map((one) => one.id)).toEqual(["q-other"]);
  expect(told).toHaveBeenCalled();
});

it("reads a queue it cannot make sense of as an empty one", () => {
  storage.kept.set(QUEUED_KEY, "{not json");
  expect(new WiredStore({} as BridgeApi).draft.queued!()).toEqual([]);
  storage.kept.set(QUEUED_KEY, JSON.stringify([{ id: "q1" }, { id: "q2", ...IDEA }]));
  expect(new WiredStore({} as BridgeApi).draft.queued!().map((one) => one.id)).toEqual(["q2"]);
});

// ---- A Session that waits on another -----------------------------------------------------------------------

const PLAN = {
  version: 1,
  recorded_at: AT,
  summary: "",
  approved: true,
  scope: [],
  groups: [{ id: "g1", agent: "rust-engineer", title: "Schema", paths: [], tasks: [{ id: "t1", text: "Add the column", state: "open" as const }] }],
};
const WAIT = { on: { id: "a", title: "Retry loop" }, condition: { kind: "ends" as const }, base: "main" as const, brief: "Write the docs", by: "you" as const };

/** A window holding the Session `@Retry loop` names, with a plan, and a Session that waits on it; Fleet answers as the arguments say. */
async function waiting(reading: { condition: object; base: string; needs?: string; plain?: boolean } = { condition: { kind: "pr_lands" }, base: "main", needs: "the schema" }) {
  const readSessionWait = vi.fn(async () => ({ ok: true as const, value: reading }));
  const startSession = vi.fn(async () => ({ ok: true as const, value: { ...record("w"), waits: WAIT } }));
  const changeWaiting = vi.fn(async () => ({ ok: true as const, value: WAIT }));
  const startWaitingNow = vi.fn(async () => ({ ok: true as const, value: record("w") }));
  let wire: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof wire) => ((wire = on), () => undefined), readSessionWait, startSession, changeWaiting, startWaitingNow } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  await new Promise((done) => setTimeout(done, 0));
  wire({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [{ ...record("a"), title: "Retry loop", plan: PLAN }, { ...record("w"), title: "Docs", waits: WAIT }] } }), seq: 0 });
  return { store, readSessionWait, startSession, changeWaiting, startWaitingNow };
}

it("reads a brief that tags no Session as no wait, and asks Fleet nothing", async () => {
  const { store, readSessionWait } = await waiting();
  expect(store.draft.readsWait!("Write the docs")).toBeUndefined();
  expect(await store.draft.settlesWait!("Write the docs")).toBeUndefined();
  expect(readSessionWait).not.toHaveBeenCalled();
});

it("draws nothing for a tagged brief until the proposer answers, asks it once the words pause, and draws its reading when it answers", async () => {
  const { store, readSessionWait } = await waiting();
  const told = vi.fn();
  store.draft.subscribe(told);
  const words = "Write the docs once @Retry loop lands, so the schema is in";

  expect(store.draft.readsWait!(words)).toBeUndefined();
  expect(readSessionWait).not.toHaveBeenCalled();

  await vi.waitFor(() => expect(readSessionWait).toHaveBeenCalledTimes(1));
  expect(readSessionWait).toHaveBeenCalledWith("a", words);
  await vi.waitFor(() => expect(told).toHaveBeenCalled());
  expect(store.draft.readsWait!(words)).toMatchObject({ on: { id: "a" }, when: "lands", base: "main", needs: "the schema" });
  expect(readSessionWait).toHaveBeenCalledTimes(1);
});

it("reads a tag that names no condition as plain, and a reading that could not be had as a wait", async () => {
  const { store } = await waiting({ condition: { kind: "ends" }, base: "main", plain: true });
  expect(await store.draft.settlesWait!("Look at what @Retry loop did")).toMatchObject({ on: { id: "a" }, plain: true });
  const failed = await waiting();
  failed.readSessionWait.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "not_connected" } } as never);
  expect((await failed.store.draft.settlesWait!("Look at what @Retry loop did"))?.plain).toBeUndefined();
});

it("asks about the words last typed and not the ones typed over", async () => {
  const { store, readSessionWait } = await waiting();
  store.draft.readsWait!("Docs after @Retry");
  store.draft.readsWait!("Docs after @Retry loop");
  store.draft.readsWait!("Docs after @Retry loop lands");
  await vi.waitFor(() => expect(readSessionWait).toHaveBeenCalledTimes(1));
  await new Promise((done) => setTimeout(done, 600));
  expect(readSessionWait).toHaveBeenCalledTimes(1);
  expect(readSessionWait).toHaveBeenCalledWith("a", "Docs after @Retry loop lands");
});

it("settles a send on the proposer's answer without waiting for the words to pause", async () => {
  const { store, readSessionWait } = await waiting({ condition: { kind: "group_done", group: "g1" }, base: "branch" });
  const reading = await store.draft.settlesWait!("Docs when @Retry loop has the schema done");
  expect(readSessionWait).toHaveBeenCalledTimes(1);
  expect(reading).toMatchObject({ on: { id: "a" }, when: "plan", part: { id: "g1", title: "Schema" }, base: "branch" });
});

it("starts on the defaults when the proposer could not read the brief", async () => {
  const { store, readSessionWait } = await waiting();
  readSessionWait.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "not_connected" } } as never);
  expect(await store.draft.settlesWait!("Docs after @Retry loop")).toMatchObject({ when: "ends", base: "main", needs: "" });
});

it("sends the wait with the start: the Session, the condition as a group or task, where from and what it needs", async () => {
  const { store, startSession } = await waiting();
  const on = { id: "a", title: "Retry loop" };
  await store.draft.start(undefined, "/code/armada", "Docs", undefined, { on, when: "plan", part: { id: "g1", title: "Schema" }, base: "main", needs: "the schema" });
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "Docs", undefined, { session: "a", condition: { kind: "group_done", group: "g1" }, base: "main", needs: "the schema" });

  await store.draft.start(undefined, "/code/armada", "Docs", undefined, { on, when: "plan", part: { id: "t1", title: "Add the column" }, base: "main", needs: "" });
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "Docs", undefined, { session: "a", condition: { kind: "task_done", task: "t1" }, base: "main" });

  await store.draft.start(undefined, "/code/armada", "Docs", undefined, { on, when: "pushed", base: "branch", needs: "" });
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "Docs", undefined, { session: "a", condition: { kind: "pr_pushed" }, base: "branch" });
});

it("draws a Session that waits with its wait, and changes the condition through Fleet", async () => {
  const { store, changeWaiting } = await waiting();
  expect(store.draft.get().find((one) => one.id === "w")?.waits).toMatchObject({ on: { id: "a" }, when: "ends", brief: "Write the docs" });

  store.draft.startWhen!("w", "plan", { id: "t1", title: "Add the column" });
  await vi.waitFor(() => expect(changeWaiting).toHaveBeenCalledWith({ waiting_session: "w", condition: { kind: "task_done", task: "t1" } }));
  store.draft.startWhen!("w", "lands");
  await vi.waitFor(() => expect(changeWaiting).toHaveBeenLastCalledWith({ waiting_session: "w", condition: { kind: "pr_lands" } }));
});

it("starts a waiting Session now through Fleet", async () => {
  const { store, startWaitingNow } = await waiting();
  store.draft.startNow!("w");
  await vi.waitFor(() => expect(startWaitingNow).toHaveBeenCalledWith("w"));
});

// ---- One file of a write clash ------------------------------------------------------------------------------

const noRoute = { ok: false as const, outcome: { ok: false as const, why: "transport" as const, detail: "404", fault: { method: "GET", path: "/clashes/file", why: "unanswerable" as const, status: 404 } } };

it("reads one file of a clash through Fleet, each side as its patch", async () => {
  const readClashFile = vi.fn(async () => ({ ok: true as const, value: { ours: "diff --git a/f b/f\n", theirs: "" } }));
  const store = new WiredStore({ readClashFile } as unknown as BridgeApi);
  expect(await store.draft.clashFile!("a", { kind: "session", id: "b" }, "f.txt")).toEqual({ ok: true, ours: "diff --git a/f b/f\n", theirs: "" });
  expect(readClashFile).toHaveBeenCalledWith("a", { kind: "session", id: "b" }, "f.txt");
});

it("says so, and does not throw, when the Fleet it reads has no route for a clash's file", async () => {
  const store = new WiredStore({ readClashFile: async () => noRoute } as unknown as BridgeApi);
  const read = await store.draft.clashFile!("a", { kind: "main", id: "main" }, "f.txt");
  expect(read).toEqual({ ok: false, said: "This Fleet cannot show the files of a clash." });
});

it("words a clash that no longer stands in Fleet's own sentence", async () => {
  const refused = { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code: "fleet.clash_not_standing", message: "the clash over `f.txt` no longer stands" } } };
  const store = new WiredStore({ readClashFile: async () => refused } as unknown as BridgeApi);
  expect(await store.draft.clashFile!("a", { kind: "session", id: "b" }, "f.txt")).toEqual({ ok: false, said: "the clash over `f.txt` no longer stands" });
});

// ---- Not yet on a plan ---------------------------------------------------------------------------------------

it("sets a plan aside by dismissing the item Fleet holds for it, `plan:<version>`, as the Needs you row dismisses one", async () => {
  const dismissWaiting = vi.fn(async () => ({ ok: true as const, value: {} }));
  let wire: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof wire) => ((wire = on), () => undefined), dismissWaiting } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  await new Promise((done) => setTimeout(done, 0));
  wire({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [{ ...record("a"), plan: { ...PLAN, version: 3, approved: false } }] } }), seq: 0 });

  expect(store.draft.work?.setAside).toBeDefined();
  store.draft.work!.setAside!("a");
  expect(dismissWaiting).toHaveBeenCalledWith({ session_id: "a", item_id: "plan:3" });
});
