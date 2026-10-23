// A Session's work over a real Fleet: the record it reads and the presses that go back out. The store is held without a window and
// its Bridge API is a spy, so what is read is what Fleet would have sent and what is asserted is the request that would be made.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionRecord, TimelineEvent, WaitingItem, WorkflowSummary } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import { NOTHING_YET } from "../../../shared/bridge";
import type { BridgeState } from "../../../shared/bridge";
import { WiredStore } from "../sessions-wired";

const AT = "2026-10-16T09:00:00Z";

const FEATURE = { id: "feature", name: "Feature", version: 1, manifest_id: "armada", steps: [{ step_id: "plan", label: "Plan", checks: [], judge_checks: [] }] } as unknown as WorkflowSummary;

const record = (change: Partial<SessionRecord> = {}): SessionRecord => ({
  id: "s1",
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
  workflow: "feature",
  proposal: { picked: ["workflow", "model", "effort"] },
  plan: { version: 3, recorded_at: AT, summary: "", approved: false, scope: [], groups: [{ id: "g1", agent: "rust-engineer", title: "Retry", paths: [], tasks: [{ id: "t1", text: "Back off", state: "open" }] }] },
  ...change,
});

const ok = { ok: true, value: {} };

// The store reads its queued ideas from the window's storage and listens for another window writing them; none is queued here.
beforeEach(() => {
  vi.stubGlobal("window", {
    localStorage: { getItem: () => null, setItem: () => undefined, removeItem: () => undefined },
    addEventListener: () => undefined,
    removeEventListener: () => undefined,
  });
});
afterEach(() => vi.unstubAllGlobals());

function held(sessions: SessionRecord[] = [record()], settings: unknown = null) {
  const api = {
    approvePlan: vi.fn(async () => ok),
    sendPlanChanges: vi.fn(async () => ok),
    setSessionWorkflow: vi.fn(async () => ok),
    sendConcern: vi.fn(async () => ok),
    dismissConcern: vi.fn(async () => ok),
    judgeSession: vi.fn(async () => ok),
  };
  const state = {
    ...NOTHING_YET,
    sessions: { state: "read", sessions },
    holds: { ...NOTHING_YET.holds, workflows: [FEATURE], manifests: [], models: null, repositories: [] },
    settings,
  } as unknown as BridgeState;
  const store = new WiredStore({ ...api, syncState: async () => ({ full: state, seq: 0 }), subscribeChanges: () => () => undefined } as unknown as BridgeApi);
  const stop = store.attach();
  return { api, store, stop };
}

const ready = async (sessions?: SessionRecord[], settings?: unknown) => {
  const one = held(sessions, settings);
  await vi.waitFor(() => expect(one.store.draft.get()).toHaveLength(1));
  return one;
};

describe("a Session's plan, over Fleet", () => {
  it("approves the version he read", async () => {
    const { api, store, stop } = await ready();
    store.draft.work!.approve("s1");
    expect(api.approvePlan).toHaveBeenCalledWith({ session_id: "s1", version: 3 });
    stop();
  });

  it("sends his marks as Fleet takes them, and approves with them when asked", async () => {
    const { api, store, stop } = await ready();
    store.draft.work!.suggest("s1", [{ kind: "add", group: "g1", task: "t-new-1", id: "t-new-1", text: "Log it" }, { kind: "agent", group: "g1", from: "rust-engineer", to: "general-purpose" }]);
    expect(api.sendPlanChanges).toHaveBeenLastCalledWith({
      session_id: "s1",
      version: 3,
      marks: [{ kind: "add_task", group: "g1", id: "t-new-1", text: "Log it" }, { kind: "subagent", group: "g1", from: "rust-engineer", to: "general-purpose" }],
    });
    store.draft.work!.suggest("s1", [{ kind: "note", id: "n1", group: "g1", text: "Why?" }], true);
    expect(api.sendPlanChanges).toHaveBeenLastCalledWith(expect.objectContaining({ approve: true }));
    stop();
  });

  it("sends nothing for a Session with no plan", async () => {
    const { api, store, stop } = await ready([record({ plan: undefined })]);
    store.draft.work!.approve("s1");
    store.draft.work!.suggest("s1", []);
    expect(api.approvePlan).not.toHaveBeenCalled();
    expect(api.sendPlanChanges).not.toHaveBeenCalled();
    stop();
  });
});

describe("the workflow and the proposer's picks, over Fleet", () => {
  it("lists Fleet's workflows by id, the same array until Fleet's list changes", async () => {
    const { store, stop } = await ready();
    const first = store.draft.work!.workflows;
    expect(first).toEqual([{ id: "feature", name: "Feature", steps: ["Plan"], manifestId: "armada" }]);
    expect(store.draft.work!.workflows).toBe(first);
    stop();
  });

  it("sets a workflow by id, or none with no id", async () => {
    const { api, store, stop } = await ready();
    store.draft.work!.choose("s1", "feature");
    expect(api.setSessionWorkflow).toHaveBeenLastCalledWith("s1", "feature");
    store.draft.work!.choose("s1", null);
    expect(api.setSessionWorkflow).toHaveBeenLastCalledWith("s1", undefined);
    stop();
  });

  it("marks the picks until he changes one, and a workflow he sets loses its mark at once", async () => {
    const { store, stop } = await ready();
    expect(store.draft.get()[0]?.work?.proposal?.picked).toEqual(["workflow", "model", "effort"]);
    store.draft.work!.unpick("s1", "model");
    expect(store.draft.get()[0]?.work?.proposal?.picked).toEqual(["workflow", "effort"]);
    store.draft.work!.choose("s1", "feature");
    expect(store.draft.get()[0]?.work?.proposal?.picked).toEqual(["effort"]);
    stop();
  });

  it("takes a value he changed off the timeline into the thread, and ignores the rest", async () => {
    const { store, stop } = await ready([record({ plan: undefined })]);
    const change = { id: 4, at: "2026-10-16T09:05:00Z", kind: "session.model", subject: { kind: "session", id: "s1" }, tone: "idle", says: "model: opus → sonnet" } as TimelineEvent;
    const noise = { id: 5, at: "2026-10-16T09:06:00Z", kind: "session.edited", subject: { kind: "session", id: "s1" }, tone: "run", says: "Edited 1 file" } as TimelineEvent;
    let heard = 0;
    const off = store.subscribe(() => (heard += 1));
    store.setEvents([noise]);
    expect(heard).toBe(0);
    store.setEvents([change, noise]);
    expect(heard).toBe(1);
    store.setEvents([change]);
    expect(heard).toBe(1);
    off();
    stop();
  });
});

describe("a judge, over Fleet", () => {
  const concern = (id: string): WaitingItem => ({ id, text: "No ceiling", since: AT, source: "judge", concern: { from: { kind: "step", index: 1, label: "Build" } }, cites: [{ path: "a/b.rs", line: 4 }] });

  it("reads concerns off the record, and sends or dismisses one by its item", async () => {
    const { api, store, stop } = await ready([record({ waiting_for: [concern("judge:1")] })]);
    expect(store.draft.get()[0]?.work?.concerns).toEqual([{ id: "judge:1", step: "Build", text: "No ceiling", cites: ["a/b.rs:4"] }]);
    store.draft.work!.sendConcern("s1", "judge:1");
    expect(api.sendConcern).toHaveBeenLastCalledWith({ session_id: "s1", item_id: "judge:1" });
    store.draft.work!.dismissConcern("s1", "judge:1");
    expect(api.dismissConcern).toHaveBeenLastCalledWith({ session_id: "s1", item_id: "judge:1" });
    stop();
  });

  it("asks for a reading and leaves its outcome to the record", async () => {
    const { api, store, stop } = await ready();
    await store.draft.work!.judge!("s1");
    expect(api.judgeSession).toHaveBeenCalledWith({ session_id: "s1" });
    expect(store.draft.get()[0]?.work?.judging).toBeUndefined();
    stop();
  });

  it("carries a reading in progress and the last one off the record", async () => {
    const { store, stop } = await ready([record({ judging: true, judge: { at: "2026-10-16T09:05:00Z", concerns: 0 } })]);
    expect(store.draft.get()[0]?.work).toMatchObject({ judging: true, judged: { at: "2026-10-16T09:05:00Z", concerns: 0 } });
    stop();
  });

  it("does not list a settled concern as waiting", async () => {
    const settled: WaitingItem = { id: "judge:2", text: "Late", since: AT, source: "judge", concern: { from: { kind: "tile" }, settled: { how: "dismissed", at: AT } } };
    const { store, stop } = await ready([record({ settled: [settled] })]);
    expect(store.draft.get()[0]?.work?.concerns).toBeUndefined();
    stop();
  });
});

describe("the setting that turns workflows off, over Fleet", () => {
  const setting = (value: boolean) => ({ settings: [{ key: "features.workflows", value, saved: null }] });

  it("draws the rail while it is on, or Fleet has not said", async () => {
    for (const settings of [null, setting(true)]) {
      const { store, stop } = await ready([record({ workflow_step: 0 })], settings);
      expect(store.draft.get()[0]?.work?.workflow?.at).toBe(0);
      expect(store.draft.get()[0]?.work?.railOff).toBeUndefined();
      stop();
    }
  });

  it("hides the steps, the picker and the proposer's mark for a workflow while it is off", async () => {
    const { store, stop } = await ready([record({ workflow_step: 0 })], setting(false));
    const work = store.draft.get()[0]?.work;
    expect(work?.railOff).toBe(true);
    expect(work?.workflow).toBeUndefined();
    expect(work?.proposal?.picked).toEqual(["model", "effort"]);
    stop();
  });
});
