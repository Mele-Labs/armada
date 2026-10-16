// What the store behind the window's Sessions does with what main publishes.

import { expect, it } from "vitest";
import type { SessionRecord } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { NOTHING_YET } from "../../shared/bridge";
import type { BridgeState } from "../../shared/bridge";
import type { StateChange } from "../../shared/state-change";
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
