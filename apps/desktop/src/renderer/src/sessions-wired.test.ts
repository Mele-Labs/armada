// What the store behind the window's Sessions does with what main publishes.

import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { AppliedRule, JobLifecycles, SessionRecord } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import type { StateChange } from "../../shared/state-change";
import { forgetKept, writeKept } from "@armada/components";
import { inFlight } from "@armada/components/src/compositions/LaneCanvas/build";
import { aimKey } from "./session-view/mod-aim";
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

/** A store with one open follow-up, whose Fleet answers the two thumbs as `answer` says, with the window's refusals caught in `told`. */
function thumbed(answer: "ok" | "refused") {
  const refusal = { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code: "fleet.later_not_open", message: "that follow-up is not open" } } };
  const settleLater = vi.fn(async () => (answer === "ok" ? { ok: true as const, value: {} } : refusal));
  const confirmLater = vi.fn(async () => (answer === "ok" ? { ok: true as const, value: record("a") } : refusal));
  const told = vi.fn();
  vi.stubGlobal("window", { ...(window as object), dispatchEvent: told });
  vi.stubGlobal("CustomEvent", class { constructor(readonly type: string, readonly init: { detail: unknown }) {} get detail() { return this.init.detail; } });
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof send) => ((send = on), () => undefined), settleLater, confirmLater } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  const later = [{ id: "later-2", sentence: "The Studio icon tooltip is still owed.", said_at: AT, confirmed: true }];
  send({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [{ ...record("a"), later }] } }), seq: 0 });
  return { store, settleLater, confirmLater, said: () => told.mock.calls.map(([event]) => (event as { detail: { sentence: string } }).detail.sentence) };
}

it("reads a thumbs up Fleet kept off the record as confirmed", () => {
  const { store } = thumbed("ok");
  expect(store.draft.get()[0]?.later).toEqual([{ id: "later-2", text: "The Studio icon tooltip is still owed.", at: AT, confirmed: true }]);
});

it("sends thumbs down as a dropped follow-up that was misread, and a plain Drop without it", () => {
  const { store, settleLater } = thumbed("ok");
  store.draft.dropLater!("a", "later-2", true);
  expect(settleLater).toHaveBeenLastCalledWith({ session_id: "a", id: "later-2", how: "dropped", misread: true });
  store.draft.dropLater!("a", "later-2");
  expect(settleLater).toHaveBeenLastCalledWith({ session_id: "a", id: "later-2", how: "dropped" });
});

it("sends thumbs up to Fleet by Session and follow-up id", () => {
  const { store, confirmLater } = thumbed("ok");
  store.draft.confirmLater!("a", "later-2");
  expect(confirmLater).toHaveBeenCalledWith({ session_id: "a", id: "later-2" });
});

it("says Fleet's words when it refuses either thumb", async () => {
  const { store, said } = thumbed("refused");
  store.draft.confirmLater!("a", "later-2");
  store.draft.dropLater!("a", "later-2", true);
  await new Promise((done) => setTimeout(done, 0));
  expect(said()).toEqual(["that follow-up is not open", "that follow-up is not open"]);
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

const FIRST = { text: "a dark theme", files: [], sketches: [], tags: [] };

it("sends an empty Mod Job's aim with its first message and not with the next", async () => {
  const { store, sendSessionMessage } = await starting("ok");
  writeKept(aimKey("new"), "theme");

  expect(await store.draft.send("new", FIRST)).toBe(true);
  expect(sendSessionMessage).toHaveBeenLastCalledWith({ session_id: "new", text: "a dark theme", aim: "theme" });

  expect(await store.draft.send("new", { ...FIRST, text: "darker" })).toBe(true);
  expect(sendSessionMessage).toHaveBeenLastCalledWith({ session_id: "new", text: "darker" });
});

it("sends no aim where none was picked, and keeps the pick when Fleet refuses the message", async () => {
  const { store, sendSessionMessage } = await starting("ok");
  expect(await store.draft.send("new", FIRST)).toBe(true);
  expect(sendSessionMessage).toHaveBeenLastCalledWith({ session_id: "new", text: "a dark theme" });

  writeKept(aimKey("new"), "source");
  // A refusal is told as a toast, which the stubbed window has to be able to hear.
  (window as unknown as { dispatchEvent: () => boolean }).dispatchEvent = () => true;
  sendSessionMessage.mockResolvedValueOnce({ ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } } as never);
  expect(await store.draft.send("new", FIRST)).toBe(false);
  expect(await store.draft.send("new", FIRST)).toBe(true);
  expect(sendSessionMessage).toHaveBeenLastCalledWith({ session_id: "new", text: "a dark theme", aim: "source" });
  forgetKept(aimKey("new"));
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

// ---- A Job held on its start card ----------------------------------------------------------------------------

/** A store whose Fleet takes every act the start card makes, with the Job already in the window; `begun` is what Fleet answers Start. */
async function onItsCard(begun: "ok" | "refused" = "ok") {
  vi.stubGlobal("window", { ...(window as object), dispatchEvent: vi.fn() });
  vi.stubGlobal("CustomEvent", class { constructor(readonly type: string, readonly init: { detail: unknown }) {} });
  const ok = async (..._args: unknown[]) => ({ ok: true as const, value: record("new") });
  const startSession = vi.fn(ok);
  const beginSession = vi.fn(async () => (begun === "ok" ? { ok: true as const, value: record("new") } : { ok: false as const, outcome: { ok: false as const, why: "not_connected" as const } }));
  const renameSession = vi.fn(ok);
  const nameSession = vi.fn(ok);
  const tuneSession = vi.fn(ok);
  const setSessionWorkflow = vi.fn(ok);
  let wire: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof wire) => ((wire = on), () => undefined), startSession, beginSession, renameSession, nameSession, tuneSession, setSessionWorkflow } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  await new Promise((done) => setTimeout(done, 0));
  wire({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [record("new")] } }), seq: 0 });
  return { store, startSession, beginSession, renameSession, nameSession, tuneSession, setSessionWorkflow };
}

it("starts a Job held on its card only when Dispatch says so, and never one that waits on another", async () => {
  const { store, startSession } = await onItsCard();
  await store.draft.start(undefined, "/code/armada", "fix it", {}, undefined, true);
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "fix it", {}, undefined, true);
  await store.draft.start(undefined, "/code/armada", "fix it", {});
  expect(startSession).toHaveBeenLastCalledWith(undefined, "/code/armada", "fix it", {});
  await store.draft.start(undefined, "/code/armada", "fix it", {}, { on: { id: "a", title: "Retry loop" }, when: { kind: "ends" }, base: "main", needs: "" } as never, true);
  expect(startSession.mock.calls.at(-1)).toHaveLength(5);
  expect(startSession.mock.calls.at(-1)![4]).toMatchObject({ session: "a" });
});

it("begins the Job with the delivery chosen on the card, or with none where its kind has no landing step", async () => {
  const { store, beginSession } = await onItsCard();
  expect(await store.draft.begin!("new", "open_pr")).toBe(true);
  expect(beginSession).toHaveBeenLastCalledWith({ session_id: "new", delivery: "open_pr" });
  expect(await store.draft.begin!("new")).toBe(true);
  expect(beginSession).toHaveBeenLastCalledWith({ session_id: "new" });
  expect(await store.draft.begin!("new", "keep_local", "Port the loader first")).toBe(true);
  expect(beginSession).toHaveBeenLastCalledWith({ session_id: "new", delivery: "keep_local", message: "Port the loader first" });
});

it("says Start was refused and does not count the Job as begun", async () => {
  const { store } = await onItsCard("refused");
  expect(await store.draft.begin!("new", "keep_local")).toBe(false);
});

it("has Fleet name a Job from its first message, and settles when Fleet answers", async () => {
  const { store, nameSession } = await onItsCard();
  await store.draft.nameForMe!("new");
  expect(nameSession).toHaveBeenCalledWith({ session_id: "new" });
});

it("sends the card's title, kind, model and effort through rename, workflow and tune, with the mode unchanged", async () => {
  const { store, renameSession, setSessionWorkflow, tuneSession } = await onItsCard();
  store.draft.rename!("new", "Retry the store client");
  expect(renameSession).toHaveBeenCalledWith({ session_id: "new", title: "Retry the store client" });
  store.draft.work!.choose("new", "bug");
  expect(setSessionWorkflow).toHaveBeenCalledWith("new", "bug");
  store.draft.tune("new", { model: "opus", effort: "high", mode: "plan" });
  expect(tuneSession).toHaveBeenCalledWith({ session_id: "new", model: "opus", effort: "high", mode: "plan" });
  store.draft.tune("new", { model: null, effort: null, mode: "auto" });
  expect(tuneSession).toHaveBeenLastCalledWith({ session_id: "new", mode: "auto" });
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
  groups: [{ id: "g1", agent: "rust-engineer", title: "Schema", paths: [], lands_in: { label: "PR 1" }, tasks: [{ id: "t1", text: "Add the column", state: "open" as const }] }],
};
const WAIT = { on: { id: "a", title: "Retry loop" }, condition: { kind: "ends" as const }, base: "main" as const, brief: "Write the docs", by: "you" as const, held: true };

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

it("approves a waiting Session and sets what it starts on through Fleet, and draws what Fleet kept", async () => {
  const { store, changeWaiting } = await waiting();
  store.draft.startThen!("w", false);
  await vi.waitFor(() => expect(changeWaiting).toHaveBeenCalledWith({ waiting_session: "w", held: false }));
  store.draft.startTuning!("w", { model: "opus", effort: null });
  await vi.waitFor(() => expect(changeWaiting).toHaveBeenLastCalledWith({ waiting_session: "w", model: "opus", effort: "" }));
  store.draft.startTuning!("w", { workflow: "feature" });
  await vi.waitFor(() => expect(changeWaiting).toHaveBeenLastCalledWith({ waiting_session: "w", workflow: "feature" }));
});

it("draws the approval and picks the wait holds", async () => {
  const { store } = await waiting();
  expect(store.draft.get().find((one) => one.id === "w")?.waits).toMatchObject({ held: true });
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

// ---- Reopening what he closed from Needs you ------------------------------------------------------------------

/** A store whose one Job holds a closed item, with Fleet's answer to a Reopen as `answer` says and the window's refusals caught in `told`. */
function reopening(answer: "ok" | "refused") {
  const refusal = { ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code: "fleet.session_waiting_unreopenable", message: "the call it asked about has ended" } } };
  const reopenWaiting = vi.fn(async () => (answer === "ok" ? { ok: true as const, value: record("a") } : refusal));
  const told = vi.fn();
  vi.stubGlobal("window", { ...(window as object), dispatchEvent: told });
  vi.stubGlobal("CustomEvent", class { constructor(readonly type: string, readonly init: { detail: unknown }) {} get detail() { return this.init.detail; } });
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: (on: typeof send) => ((send = on), () => undefined), reopenWaiting } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  const closed = [{ item: { id: "a2", text: "Open the walk", since: AT, source: "walk" as const }, at: AT }];
  send({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: [{ ...record("a"), closed }] } }), seq: 0 });
  return { store, reopenWaiting, said: () => told.mock.calls.map(([event]) => (event as { detail: { sentence: string } }).detail.sentence) };
}

it("reads what he closed off the record, so the Closed rows have something to draw", () => {
  const { store } = reopening("ok");
  expect(store.draft.get()[0]?.closed).toEqual([{ item: { id: "a2", text: "Open the walk", since: AT, source: "walk" }, at: AT }]);
});

it("sends Reopen to Fleet by the Job and the item", () => {
  const { store, reopenWaiting } = reopening("ok");
  store.draft.reopenWaiting!("a", "a2");
  expect(reopenWaiting).toHaveBeenCalledExactlyOnceWith({ session_id: "a", item_id: "a2" });
});

it("tells Fleet's refusal of a Reopen as a toast in its own words", async () => {
  const { store, said } = reopening("refused");
  await store.draft.reopenWaiting!("a", "a2");
  expect(said()).toEqual(["the call it asked about has ended"]);
});

// ---- A pull request press frees on the write's answer --------------------------------------------------------

it("answers a pull request press on the write, while the read that refreshes the sheet is still out", async () => {
  const pressPullRequest = vi.fn((_id: string, _number: number, press: string) => (press === "read" ? new Promise<never>(() => undefined) : Promise.resolve({ ok: true as const, value: {} })));
  const store = new WiredStore({ pressPullRequest } as unknown as BridgeApi);
  const answered = await Promise.race([store.draft.act!("a", 7, "ready"), new Promise<string>((done) => setTimeout(() => done("held"), 50))]);
  expect(answered).toBe(true);
  expect(pressPullRequest).toHaveBeenCalledWith("a", 7, "read");
});

it("holds the picked repository's Sessions in the list and every repository's beside it, which the palette jumps to", () => {
  const { store, publish } = fed();
  const repositories = [
    { root: "/repo", manifest: { id: "armada" } },
    { root: "/pocket", manifest: { id: "pocket" } },
  ] as unknown as NonNullable<BridgeState["holds"]["repositories"]>;
  publish({
    repository: "/repo",
    holds: { ...NOTHING_YET.holds, repositories },
    sessions: { state: "read", sessions: [record("a"), { ...record("b"), manifest_id: "pocket" }] },
  });
  expect(store.draft.get().map((one) => one.id)).toEqual(["a"]);
  const everywhere = store.draft.everywhere!();
  expect(everywhere.map((one) => one.id)).toEqual(["a", "b"]);
  // The same Session, so a row that did not change is not drawn again.
  expect(everywhere[0]).toBe(store.draft.get()[0]);
  publish({ repository: null as unknown as string, holds: { ...NOTHING_YET.holds, repositories }, sessions: { state: "read", sessions: [record("a"), { ...record("b"), manifest_id: "pocket" }] } });
  expect(store.draft.everywhere!()).toBe(store.draft.get());
});

// ---- A start refused from the dispatch sheet ----------------------------------------------------------------

/** A store whose Fleet refuses every start with a project row it does not know, with the window's events caught in `heard`. */
async function refusedStart() {
  const heard: { type: string; detail: unknown }[] = [];
  vi.stubGlobal("window", { ...(window as object), dispatchEvent: (event: { type: string; detail: unknown }) => heard.push(event) });
  vi.stubGlobal("CustomEvent", class { constructor(readonly type: string, readonly init: { detail: unknown }) {} get detail() { return this.init.detail; } });
  const error = { code: "fleet.no_such_project_row", message: "No such row in your projects.", run_id: "r1", fields: { row: "gone" }, chain: [] };
  const startSession = vi.fn(async () => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error } }));
  const api = { syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }), subscribeChanges: () => () => undefined, startSession } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  await new Promise((done) => setTimeout(done, 0));
  return { store, startSession, raised: () => heard.filter((event) => event.type === "armada:raise-outcome").map((event) => event.detail as { error: { code: string; fields: Record<string, string> } }) };
}

it("raises a start the dispatch sheet made and Fleet refused as the window's toast, with Fleet's code and the request as its fields, and leaves the Sessions line unset", async () => {
  const { store, startSession, raised } = await refusedStart();

  const id = await store.draft.start(undefined, "/code/armada", "fix it", undefined, undefined, undefined, [{ row: "gone" }], true);

  expect(id).toBeUndefined();
  expect(startSession).toHaveBeenCalledWith(undefined, "/code/armada", "fix it", undefined, undefined, undefined, [{ row: "gone" }]);
  const [outcome] = raised();
  expect(raised()).toHaveLength(1);
  expect(outcome?.error.code).toBe("fleet.no_such_project_row");
  expect(outcome?.error.fields).toEqual({ row: "gone", act: "start_session", root: "/code/armada", request: "fix it", project_rows: JSON.stringify([{ row: "gone" }]) });
  expect(store.draft.said!()).toBeUndefined();
});

it("still says a start refused anywhere else on the Sessions line, and raises no toast for it", async () => {
  const { store, raised } = await refusedStart();

  await store.draft.start(undefined, "/code/armada", "fix it");

  expect(store.draft.said!()).toBe("No such row in your projects.");
  expect(raised()).toEqual([]);
});

// ---- What a Job holds -----------------------------------------------------------------------------------------------------

const TICKET: JobLifecycles["lifecycles"][number] = {
  id: "ticket",
  name: "Ticket",
  scope: { kind: "everywhere" },
  states: [
    { id: "todo", label: "To do", read: { source: "tracker", words: ["To Do"] } },
    { id: "in_progress", label: "In progress", read: { source: "tracker", words: ["In Progress"] } },
  ],
  transitions: [{ from: "todo", to: "in_progress" }],
  ends: [],
};

const holding = (id: string, state: string): JobLifecycles => ({
  session_id: id,
  lifecycles: [TICKET],
  instances: [{ lifecycle: "ticket", key: "ARM-41", label: "ARM-41", standing: { kind: "in", state }, links: [] }],
  moves: [],
});

/** A store whose Fleet answers `get_job_lifecycles` with `answer` and `list_lifecycle_rules` with `rules`, and whose `job_lifecycles.changed` is sent by hand. */
async function holdingStore(answer: (id: string) => ReturnType<BridgeApi["getJobLifecycles"]>, rules?: () => ReturnType<BridgeApi["listLifecycleRules"]>) {
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  let changed: (held: JobLifecycles) => void = () => undefined;
  const getJobLifecycles = vi.fn(answer);
  const listLifecycleRules = vi.fn(rules ?? (async () => ({ ok: true as const, value: { manifest_id: "armada", rules: [], left_out: [] } })));
  const api = {
    syncState: () => Promise.resolve({ full: NOTHING_YET, seq: 0 }),
    subscribeChanges: (on: typeof send) => ((send = on), () => undefined),
    getJobLifecycles,
    listLifecycleRules,
    onJobLifecyclesChanged: (on: typeof changed) => ((changed = on), () => undefined),
  } as unknown as BridgeApi;
  const store = new WiredStore(api);
  store.attach();
  const publish = async (...records: SessionRecord[]) => {
    send({ full: structuredClone({ ...NOTHING_YET, sessions: { state: "read", sessions: records } }), seq: 0 });
    await new Promise((done) => setTimeout(done, 0));
  };
  return { store, getJobLifecycles, listLifecycleRules, publish, changed: (held: JobLifecycles) => changed(held) };
}

it("draws a lane for each instance a Job holds, as Fleet reads it, and moves it when Fleet says it moved", async () => {
  const { store, publish, changed } = await holdingStore(async (id) => ({ ok: true, value: holding(id, "todo") }));
  await publish(record("a"));
  expect(store.draft.get()[0]?.lanes?.job.instances).toMatchObject([{ label: "ARM-41", state: "todo" }]);

  changed(holding("a", "in_progress"));
  expect(store.draft.get()[0]?.lanes?.job.instances).toMatchObject([{ label: "ARM-41", state: "in_progress" }]);
});

it("keeps what an event said when an older read answers after it", async () => {
  let answer: (held: JobLifecycles) => void = () => undefined;
  const { store, publish, changed } = await holdingStore(() => new Promise((done) => (answer = (held) => done({ ok: true, value: held }))));
  await publish(record("a"));
  changed(holding("a", "in_progress"));
  answer(holding("a", "todo"));
  await new Promise((done) => setTimeout(done, 0));
  expect(store.draft.get()[0]?.lanes?.job.instances[0]?.state).toBe("in_progress");
});

it("draws no lanes for a Job that holds nothing, one that has ended, or a Fleet that refuses the read, and asks once for each Job", async () => {
  const empty: JobLifecycles = { session_id: "a", lifecycles: [], instances: [], moves: [] };
  const { store, publish, getJobLifecycles } = await holdingStore(async (id) => (id === "b" ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: { ...empty, session_id: id } }));
  await publish(record("a"), record("b"), { ...record("c"), state: "ended" });
  await publish(record("a"), record("b"), { ...record("c"), state: "ended" });
  expect(store.draft.get().map((one) => one.lanes)).toEqual([undefined, undefined, undefined]);
  expect(getJobLifecycles.mock.calls.map(([id]) => id)).toEqual(["a", "b"]);
});

const lifecycle = (id: string, name: string, states: [string, string][], ends: string[]): JobLifecycles["lifecycles"][number] => ({
  id,
  name,
  scope: { kind: "everywhere" },
  states: states.map(([state, label]) => ({ id: state, label, read: { source: "tracker", words: [label] } })),
  transitions: [],
  ends,
});

/** A Job holding a deploy rolling out, a ticket in review, a pull request approved, a ticket whose status nobody mapped, one not read yet, and a deploy that is healthy. */
const flying = (id: string): JobLifecycles => ({
  session_id: id,
  lifecycles: [
    lifecycle("deploy", "Deploy", [["rolling_out", "Rolling out"], ["healthy", "Healthy"]], ["healthy"]),
    lifecycle("ticket", "Ticket", [["in_review", "In review"], ["closed", "Closed"]], ["closed"]),
    lifecycle("pr", "Pull request", [["approved", "Approved"], ["merged", "Merged"]], ["merged"]),
  ],
  instances: [
    { lifecycle: "deploy", key: "web", label: "Deploy", standing: { kind: "in", state: "rolling_out" }, links: [] },
    { lifecycle: "ticket", key: "ARM-42", label: "ARM-42", standing: { kind: "in", state: "in_review" }, links: [] },
    { lifecycle: "pr", key: "24", label: "PR #24", standing: { kind: "in", state: "approved" }, links: [] },
    { lifecycle: "ticket", key: "ARM-43", label: "ARM-43", standing: { kind: "unmapped", said: "Awaiting QA" }, links: [] },
    { lifecycle: "ticket", key: "ARM-44", label: "ARM-44", standing: { kind: "unread" }, links: [] },
    { lifecycle: "deploy", key: "api", label: "API", standing: { kind: "in", state: "healthy" }, links: [] },
  ],
  moves: [],
});

it("lists what End finds in flight on a real Job, as Fleet read it, and nothing once ending the Job ended it", async () => {
  const { store, publish, changed } = await holdingStore(async (id) => ({ ok: true, value: flying(id) }));
  await publish(record("a"));
  const rows = () => inFlight(store.draft.get()[0]!.lanes!.job).map((one) => (one.state === "" ? one.label : `${one.label}: ${one.state}`));
  expect(rows()).toEqual(["Deploy: Rolling out", "ARM-42: In review", "PR #24: Approved", "ARM-43: Awaiting QA", "ARM-44"]);

  const ended = flying("a");
  changed({ ...ended, instances: ended.instances.map((one) => (one.standing.kind === "in" && one.standing.state === "healthy" ? one : { ...one, ended_at: AT })) });
  expect(rows()).toEqual([]);
  expect(store.draft.get()[0]?.lanes?.job.instances.filter((one) => one.ended === true)).toHaveLength(5);
});

it("lists a ticket nobody has read yet when End finds nothing else, and a status nobody mapped", async () => {
  const { store, publish } = await holdingStore(async (id) => ({
    ok: true,
    value: {
      ...flying(id),
      instances: [
        { lifecycle: "ticket", key: "ARM-44", label: "ARM-44", standing: { kind: "unread" }, links: [] },
        { lifecycle: "ticket", key: "ARM-43", label: "ARM-43", standing: { kind: "unmapped", said: "Awaiting QA" }, links: [] },
      ],
    },
  }));
  await publish(record("a"));

  const rows = inFlight(store.draft.get()[0]!.lanes!.job).map((one) => (one.state === "" ? one.label : `${one.label}: ${one.state}`));
  expect(rows).toEqual(["ARM-44", "ARM-43: Awaiting QA"]);
});

const MOVES_TICKET: AppliedRule = {
  rule: {
    id: "move-ticket",
    name: "Move to In progress",
    scope: { kind: "everywhere" },
    when: { kind: "step_starts", step: "plan" },
    conditions: [],
    does: [{ kind: "tracker_write", autonomy: "acts", what: { act: "tracker_move", to: "in_progress", target: { among: { kind: "held", lifecycle: "ticket" } } } }],
    on_failure: { retries: 0, then: "ask_owner" },
  },
  kept: { rank: { reach: "everywhere", source: "armada" } },
};

const planned = (id: string): JobLifecycles => ({
  ...holding(id, "in_progress"),
  acts: [{ rule: "move-ticket", text: "Move ARM-41 to In progress", course: "acts", standing: "planned", at: AT, on: { lifecycle: "ticket", key: "ARM-41" } }],
});

it("reads the repository's rules for a Job whose rules planned an act, and draws the act on its rule", async () => {
  const { store, publish, listLifecycleRules } = await holdingStore(async (id) => ({ ok: true, value: planned(id) }), async () => ({ ok: true, value: { manifest_id: "armada", rules: [MOVES_TICKET], left_out: [] } }));
  await publish(record("a"));
  expect(listLifecycleRules.mock.calls).toEqual([["armada"]]);
  const job = store.draft.get()[0]?.lanes?.job;
  expect(job?.copy.rules.map((one) => one.id)).toEqual(["move-ticket"]);
  expect(job?.fired).toMatchObject([{ rule: "move-ticket", outcome: "waiting", acts: [{ text: "Move ARM-41 to In progress", standing: "planned" }] }]);
});

it("reads no rules for a Job whose rules planned nothing, and keeps the lanes where a Fleet refuses them", async () => {
  const quiet = await holdingStore(async (id) => ({ ok: true, value: holding(id, "todo") }));
  await quiet.publish(record("a"));
  expect(quiet.listLifecycleRules).not.toHaveBeenCalled();
  expect(quiet.store.draft.get()[0]?.lanes?.job.fired).toEqual([]);

  const refused = await holdingStore(async (id) => ({ ok: true, value: planned(id) }), async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }));
  await refused.publish(record("a"));
  expect(refused.store.draft.get()[0]?.lanes?.job.instances).toMatchObject([{ label: "ARM-41", state: "in_progress" }]);
  expect(refused.store.draft.get()[0]?.lanes?.job.fired).toEqual([]);
});
