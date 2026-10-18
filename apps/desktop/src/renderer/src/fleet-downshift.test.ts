import { describe, expect, it } from "vitest";

import type { ModelDownshiftState, Outcome } from "@armada/protocol";
import type { ModelDownshiftRead } from "../../shared/api/model-downshift";
import type { BridgeState } from "../../shared/bridge";
import { createFleetDownshift } from "./fleet-downshift";
import type { FleetDownshift } from "./fleet-downshift";

const OFFERED: ModelDownshiftState = { downshift: { phase: "offered", percent: 83, was: "opus", to: "sonnet" } };
const SHIFTED: ModelDownshiftState = {
  downshift: { phase: "shifted", was: "opus", to: "sonnet", until: "2026-10-18T00:00:00Z" },
};
const NONE: ModelDownshiftState = {};

/** `fleet.downshift_wrong_phase`, the 409 an act made in a phase the state has left answers with. */
const WRONG_PHASE: Outcome = {
  ok: false,
  why: "refused",
  error: { code: "fleet.downshift_wrong_phase", message: "The default is not offered." },
} as Outcome;

const read = (value: ModelDownshiftState): ModelDownshiftRead => ({ ok: true, value });
const wait = () => new Promise((resolve) => setTimeout(resolve, 0));

function fleet(held: { now: ModelDownshiftState; answers?: Partial<Record<"accept" | "restore" | "keep" | "settle", ModelDownshiftRead>> }) {
  let connection: (facts: Pick<BridgeState, "connection">) => void = () => {};
  let changed: (state: ModelDownshiftState) => void = () => {};
  const posts: string[] = [];
  let reads = 0;
  const post = (name: "accept" | "restore" | "keep" | "settle") => async (): Promise<ModelDownshiftRead> => {
    posts.push(name);
    return held.answers?.[name] ?? read(held.now);
  };
  const api: FleetDownshift = {
    state: async () => ({ connection: { state: "connecting" } as BridgeState["connection"] }),
    subscribe: (on) => ((connection = on), () => {}),
    getModelDownshift: async () => ((reads += 1), read(held.now)),
    acceptDownshift: post("accept"),
    restoreDownshift: post("restore"),
    keepDownshift: post("keep"),
    settleDownshift: post("settle"),
    onModelDownshiftChanged: (on) => ((changed = on), () => {}),
  };
  return {
    api,
    posts,
    reads: () => reads,
    stands: (state: ModelDownshiftState) => void (held.now = state),
    connect: () => connection({ connection: { state: "connected" } as BridgeState["connection"] }),
    changed: (state: ModelDownshiftState) => changed(state),
  };
}

describe("the Job model's step down over Fleet", () => {
  it("holds nothing until Fleet is connected, then the state it reads", async () => {
    const held = fleet({ now: OFFERED });
    const source = createFleetDownshift(held.api);
    source.subscribe(() => {});
    expect(source.get()).toBeUndefined();

    held.connect();
    await wait();
    expect(held.reads()).toBe(1);
    expect(source.get()).toEqual(OFFERED.downshift);
  });

  it("replaces what it holds with each state Fleet tells it, and none is none", async () => {
    const held = fleet({ now: OFFERED });
    const source = createFleetDownshift(held.api);
    const heard: unknown[] = [];
    source.subscribe(() => heard.push(source.get()));
    held.connect();
    await wait();

    held.changed({ downshift: { phase: "offered", percent: 91, was: "opus", to: "sonnet" } });
    expect(source.get()).toEqual({ phase: "offered", percent: 91, was: "opus", to: "sonnet" });
    held.changed(NONE);
    expect(source.get()).toBeUndefined();
    expect(heard).toHaveLength(3);
  });

  it("posts the accept and holds the state it answers with", async () => {
    const held = fleet({ now: OFFERED, answers: { accept: read(SHIFTED) } });
    const source = createFleetDownshift(held.api);
    source.subscribe(() => {});
    held.connect();
    await wait();

    source.accept();
    await wait();
    expect(held.posts).toEqual(["accept"]);
    expect(source.get()).toEqual(SHIFTED.downshift);
  });

  it("posts restore, keep and settle each to its own route", async () => {
    const held = fleet({ now: OFFERED, answers: { restore: read(NONE), keep: read(NONE), settle: read(NONE) } });
    const source = createFleetDownshift(held.api);
    source.subscribe(() => {});
    held.connect();
    await wait();

    source.restore();
    source.keep();
    source.settle();
    await wait();
    expect(held.posts).toEqual(["restore", "keep", "settle"]);
    expect(source.get()).toBeUndefined();
  });

  it("reads the state again when an act is refused for the phase, and shows no error", async () => {
    const held = fleet({ now: OFFERED, answers: { accept: { ok: false, outcome: WRONG_PHASE } } });
    const source = createFleetDownshift(held.api);
    source.subscribe(() => {});
    held.connect();
    await wait();
    held.stands(SHIFTED);

    source.accept();
    await wait();
    expect(held.reads()).toBe(2);
    expect(source.get()).toEqual(SHIFTED.downshift);
  });
});
