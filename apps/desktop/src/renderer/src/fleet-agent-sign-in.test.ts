import { describe, expect, it } from "vitest";

import type { AgentSignIn, Outcome } from "@armada/protocol";
import type { AgentSignInRead } from "../../shared/api/agent-sign-in";
import type { BridgeState } from "../../shared/bridge";
import { createFleetAgentSignIn } from "./fleet-agent-sign-in";
import type { FleetAgentSignIn } from "./fleet-agent-sign-in";

const SINCE = "2026-10-25T09:00:00Z";
const OUT: AgentSignIn = { signed_out_since: SINCE };
const SIGNING_IN: AgentSignIn = { signed_out_since: SINCE, signing_in: true };
const IN: AgentSignIn = {};

const REFUSED: Outcome = { ok: false, why: "refused", error: { code: "fleet.refused", message: "No." } } as Outcome;

const read = (value: AgentSignIn): AgentSignInRead => ({ ok: true, value });
const wait = () => new Promise((resolve) => setTimeout(resolve, 0));

function fleet(held: { now: AgentSignIn; press?: AgentSignInRead }) {
  let connection: (facts: Pick<BridgeState, "connection">) => void = () => {};
  let changed: (state: AgentSignIn) => void = () => {};
  let reads = 0;
  let presses = 0;
  const api: FleetAgentSignIn = {
    state: async () => ({ connection: { state: "connecting" } as BridgeState["connection"] }),
    subscribe: (on) => ((connection = on), () => {}),
    getAgentSignIn: async () => ((reads += 1), read(held.now)),
    startAgentSignIn: async () => ((presses += 1), held.press ?? read(held.now)),
    onAgentSignInChanged: (on) => ((changed = on), () => {}),
  };
  return {
    api,
    reads: () => reads,
    presses: () => presses,
    stands: (state: AgentSignIn) => void (held.now = state),
    connect: () => connection({ connection: { state: "connected" } as BridgeState["connection"] }),
    changed: (state: AgentSignIn) => changed(state),
  };
}

describe("the agents' sign-in over Fleet", () => {
  it("holds signed in until Fleet is connected, then the state it reads", async () => {
    const held = fleet({ now: OUT });
    const source = createFleetAgentSignIn(held.api);
    source.subscribe(() => {});
    expect(source.get()).toEqual({ signing_in: false });

    held.connect();
    await wait();
    expect(held.reads()).toBe(1);
    expect(source.get()).toEqual({ signed_out_since: SINCE, signing_in: false });
  });

  it("replaces what it holds with each state Fleet tells it, and reads signing_in false where the wire leaves it out", async () => {
    const held = fleet({ now: IN });
    const source = createFleetAgentSignIn(held.api);
    const heard: unknown[] = [];
    source.subscribe(() => heard.push(source.get()));
    held.connect();
    await wait();

    held.changed(OUT);
    expect(source.get()).toEqual({ signed_out_since: SINCE, signing_in: false });
    held.changed(SIGNING_IN);
    expect(source.get()).toEqual({ signed_out_since: SINCE, signing_in: true });
    held.changed(IN);
    expect(source.get()).toEqual({ signing_in: false });
    expect(heard).toHaveLength(4);
  });

  it("posts the press once and holds the state it answers with", async () => {
    const held = fleet({ now: OUT, press: read(SIGNING_IN) });
    const source = createFleetAgentSignIn(held.api);
    source.subscribe(() => {});
    held.connect();
    await wait();

    source.signIn();
    await wait();
    expect(held.presses()).toBe(1);
    expect(source.get()).toEqual({ signed_out_since: SINCE, signing_in: true });
  });

  it("reads the state again when the press is refused, and shows no error", async () => {
    const held = fleet({ now: OUT, press: { ok: false, outcome: REFUSED } });
    const source = createFleetAgentSignIn(held.api);
    source.subscribe(() => {});
    held.connect();
    await wait();
    held.stands(IN);

    source.signIn();
    await wait();
    expect(held.reads()).toBe(2);
    expect(source.get()).toEqual({ signing_in: false });
  });
});
