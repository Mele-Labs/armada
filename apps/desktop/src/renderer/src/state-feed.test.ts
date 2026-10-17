import { expect, it } from "vitest";

import type { BridgeState } from "../../shared/bridge";
import { diff, type StateChange } from "../../shared/state-change";
import { stateOf } from "./state-feed";

const held = (rows: { id: string; n: number }[], other = { x: 1 }) => ({ rows, other }) as unknown as BridgeState;

function wire(first: StateChange<BridgeState>) {
  let send: (change: StateChange<BridgeState>) => void = () => undefined;
  let syncs = 0;
  let again: StateChange<BridgeState> = first;
  const api = {
    syncState: () => (syncs++, Promise.resolve(again)),
    subscribeChanges: (on: typeof send) => ((send = on), () => undefined),
  };
  return { api, send: (change: StateChange<BridgeState>) => send(change), syncs: () => syncs, answer: (change: StateChange<BridgeState>) => (again = change) };
}

it("folds a patch onto what it holds and keeps what the patch left alone as the same object", async () => {
  const one = held([{ id: "a", n: 1 }, { id: "b", n: 1 }]);
  const two = held([{ id: "a", n: 1 }, { id: "b", n: 2 }]);
  const { api, send } = wire({ full: one, seq: 1 });
  const feed = stateOf(api as never);
  const heard: BridgeState[] = [];
  feed.subscribe((state) => heard.push(state));
  expect(await feed.state()).toBe(one);
  heard.length = 0;
  send({ patch: diff(one, two)!, from: 1, to: 2 });
  expect(heard).toHaveLength(1);
  expect(heard[0]).toEqual(two);
  const rowOf = (state: BridgeState) => (state as unknown as { rows: unknown[] }).rows[0];
  expect(rowOf(heard[0]!)).toBe(rowOf(one));
  expect((heard[0] as unknown as { other: unknown }).other).toBe((one as unknown as { other: unknown }).other);
});

it("ignores a patch for a state it has already moved past, and asks again for one that skipped ahead", async () => {
  const one = held([{ id: "a", n: 1 }]);
  const two = held([{ id: "a", n: 2 }]);
  const { api, send, syncs } = wire({ full: one, seq: 5 });
  const feed = stateOf(api as never);
  const heard: BridgeState[] = [];
  feed.subscribe((state) => heard.push(state));
  await feed.state();
  heard.length = 0;
  send({ patch: diff(one, two)!, from: 3, to: 4 });
  expect(heard).toHaveLength(0);
  expect(syncs()).toBe(1);
  send({ patch: diff(one, two)!, from: 7, to: 8 });
  expect(syncs()).toBe(2);
});
