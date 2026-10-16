// A Session's work over a real Fleet: the record it reads and the presses that go back out. The store is held without a window and
// its Bridge API is a spy, so what is read is what Fleet would have sent and what is asserted is the request that would be made.

import { describe, expect, it, vi } from "vitest";
import type { SessionRecord, TimelineEvent, WaitingItem, WorkflowSummary } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
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
  plan: { version: 3, recorded_at: AT, approved: false, scope: [], groups: [{ id: "g1", agent: "rust-engineer", title: "Retry", paths: [], tasks: [{ id: "t1", text: "Back off", state: "open" }] }] },
  ...change,
});

const ok = { ok: true, value: {} };

function held(sessions: SessionRecord[] = [record()]) {
  const api = {
    approvePlan: vi.fn(async () => ok),
    sendPlanChanges: vi.fn(async () => ok),
    setSessionWorkflow: vi.fn(async () => ok),
    actOnConcern: vi.fn(async () => ({ ok: true, value: null })),
    judgeSession: vi.fn(async () => ({ ok: true, value: { raised: 0 } })),
  };
  const state = {
    sessions: { state: "read", sessions },
    sessionThreads: {},
    jobs: [],
    holds: { workflows: [FEATURE], manifests: [], models: null, repositories: [] },
    repository: null,
    helm: { state: "none" },
  } as unknown as BridgeState;
  const store = new WiredStore({ ...api, state: async () => state, subscribe: () => () => undefined } as unknown as BridgeApi);
  const stop = store.attach();
  return { api, store, stop };
}

const ready = async (sessions?: SessionRecord[]) => {
  const one = held(sessions);
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
  const concern = (id: string): WaitingItem => ({ id, text: "No ceiling", since: AT, source: "judge", cites: [{ path: "a/b.rs", line: 4 }] });

  it("reads concerns off the record, and sends or dismisses one by its item", async () => {
    const { api, store, stop } = await ready([record({ waiting_for: [concern("c1")] })]);
    expect(store.draft.get()[0]?.work?.concerns).toEqual([{ id: "c1", text: "No ceiling", cites: ["a/b.rs:4"] }]);
    store.draft.work!.sendConcern("s1", "c1");
    expect(api.actOnConcern).toHaveBeenLastCalledWith("send", "s1", "c1");
    store.draft.work!.dismissConcern("s1", "c1");
    expect(api.actOnConcern).toHaveBeenLastCalledWith("dismiss", "s1", "c1");
    stop();
  });

  it("resolves a reading with whether it raised any", async () => {
    const { api, store, stop } = await ready();
    expect(await store.draft.work!.judge!("s1")).toBe(false);
    api.judgeSession.mockResolvedValueOnce({ ok: true, value: { raised: 2 } });
    expect(await store.draft.work!.judge!("s1")).toBe(true);
    expect(api.judgeSession).toHaveBeenCalledWith("s1");
    stop();
  });
});
