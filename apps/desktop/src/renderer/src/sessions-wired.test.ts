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
