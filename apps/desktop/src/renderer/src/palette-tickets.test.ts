import { describe, expect, test, vi } from "vitest";

import { NOTHING_YET, type BridgeState } from "../../shared/bridge";
import { PaletteTickets, STALE_MS } from "./palette-tickets";

const repositories = [
  { root: "/work/armada", manifest: { id: "m-armada" } },
  { root: "/work/pocket", manifest: { id: "m-pocket" } },
] as unknown as NonNullable<BridgeState["holds"]["repositories"]>;

const state = {
  ...NOTHING_YET,
  holds: { ...NOTHING_YET.holds, repositories },
  mergeLines: { lines: [{ root: "/work/armada", line: [], off: [], landed: [], sent_back: [], hub: { pull_requests: [{ number: 1899, title: "Cache the plan windows", branch: "b", url: "https://forge.example/armada/pull/1899" }] } }] },
} as unknown as BridgeState;

const issue = (reference: string, title: string) => ({ reference, title, url: `https://forge.example/${reference}` });

function opened(over: { lists?: unknown; issues?: Record<string, unknown> } = {}) {
  let now = 1_000_000;
  const getReviewLists = vi.fn(async () => over.lists ?? { ok: true, value: { requested: [{ manifest_id: "m-pocket", number: 212, title: "Bump the manifest icons", address: "https://forge.example/pocket/pull/212" }, { manifest_id: "m-armada", number: 1899, title: "Same one", address: "x" }], team: [], assigned: [], mine: [] } });
  const listIssues = vi.fn(async (manifestId: string) => (over.issues?.[manifestId] as never) ?? { ok: true, value: { issues: manifestId === "m-armada" ? [issue("#2214", "Plan windows refresh twice a minute"), issue("ABC-7", "Kept by a key")] : [issue("#88", "The install prompt shows the old icon")] } });
  const api = { state: async () => state, subscribe: () => () => undefined, getReviewLists, listIssues } as never;
  const store = new PaletteTickets(api, () => now);
  const stop = store.attach();
  return { store, stop, getReviewLists, listIssues, later: (ms: number) => (now += ms), settle: () => new Promise((done) => setTimeout(done, 0)) };
}

describe("what the palette lists of pull requests and issues", () => {
  test("holds the open pull requests the window already has before anything is asked", async () => {
    const { store, settle } = opened();
    await settle();
    expect(store.get().pulls).toEqual([{ repo: "armada", number: 1899, title: "Cache the plan windows", url: "https://forge.example/armada/pull/1899" }]);
    expect(store.get().issues).toEqual([]);
  });

  test("reads issues and the Reviews lists when the palette opens, each in its own repository", async () => {
    const { store, settle, getReviewLists, listIssues } = opened();
    await settle();
    expect(listIssues).not.toHaveBeenCalled();
    store.load();
    await settle();
    expect(getReviewLists).toHaveBeenCalledTimes(1);
    expect(listIssues.mock.calls.map(([id]) => id)).toEqual(["m-armada", "m-pocket"]);
    expect(store.get().issues.map((one) => `${one.repo} #${one.number}`)).toEqual(["armada #2214", "pocket #88"]);
    // A pull request on the merge line is not listed twice for being on a Reviews list too.
    expect(store.get().pulls.map((one) => `${one.repo} #${one.number}`)).toEqual(["armada #1899", "pocket #212"]);
  });

  test("leaves out an issue a tracker names by a key, which has no number to find it by", async () => {
    const { store, settle } = opened();
    await settle();
    store.load();
    await settle();
    expect(store.get().issues.some((one) => one.title === "Kept by a key")).toBe(false);
  });

  test("asks again only once what it holds is stale", async () => {
    const { store, settle, listIssues, later } = opened();
    await settle();
    store.load();
    store.load();
    later(STALE_MS - 1);
    store.load();
    await settle();
    expect(listIssues).toHaveBeenCalledTimes(2);
    later(2);
    store.load();
    await settle();
    expect(listIssues).toHaveBeenCalledTimes(4);
  });

  test("keeps what it held when a read is refused", async () => {
    const { store, settle, later } = opened();
    await settle();
    store.load();
    await settle();
    const held = store.get();
    later(STALE_MS + 1);
    const refused = { ok: false, outcome: { ok: false, why: "not_connected" } };
    (store as unknown as { api: { listIssues: unknown; getReviewLists: unknown } }).api.listIssues = async () => refused;
    (store as unknown as { api: { listIssues: unknown; getReviewLists: unknown } }).api.getReviewLists = async () => refused;
    store.load();
    await settle();
    expect(store.get()).toBe(held);
  });

  test("tells a reader only when the rows changed", async () => {
    const { store, settle, later } = opened();
    await settle();
    const heard = vi.fn();
    store.subscribe(heard);
    store.load();
    await settle();
    const calls = heard.mock.calls.length;
    later(STALE_MS + 1);
    store.load();
    await settle();
    expect(heard.mock.calls.length).toBe(calls);
  });
});
