// The agents' sign-in over Fleet: the state, read once Fleet is connected and kept current by
// `agent_sign_in.changed`, which carries it whole, and the one press that starts the sign-in. The mock has its
// own source (`mock/agent-sign-in.ts`); this one is the real wire.

import type { AgentSignIn as Wire } from "@armada/protocol";

import type { AgentSignInRead } from "../../shared/api/agent-sign-in";
import type { BridgeState } from "../../shared/bridge";
import type { AgentSignIn, AgentSignInSource } from "./agent-sign-in";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetAgentSignIn = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getAgentSignIn: () => Promise<AgentSignInRead>;
  startAgentSignIn: () => Promise<AgentSignInRead>;
  onAgentSignInChanged: (on: (state: Wire) => void) => () => void;
};

const SIGNED_IN: AgentSignIn = { signing_in: false };

/** The wire leaves empty fields out; the alert reads them whole. */
const shown = (wire: Wire): AgentSignIn =>
  wire.signed_out_since === undefined
    ? SIGNED_IN
    : { signed_out_since: wire.signed_out_since, signing_in: wire.signing_in === true };

export function createFleetAgentSignIn(fleet: FleetAgentSignIn): AgentSignInSource {
  let state: AgentSignIn = SIGNED_IN;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: Wire): void => {
    state = shown(to);
    listeners.forEach((on) => on());
  };
  const read = (): void => void fleet.getAgentSignIn().then((one) => one.ok && set(one.value));
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offChanged = fleet.onAgentSignInChanged(set);
    void fleet.state().then(connectedIn);
    stop = () => {
      offState();
      offChanged();
      wasConnected = false;
    };
  };

  return {
    get: () => state,
    subscribe: (on) => {
      listeners.add(on);
      if (stop === null) start();
      return () => {
        listeners.delete(on);
        if (listeners.size === 0 && stop !== null) {
          stop();
          stop = null;
        }
      };
    },
    /**
     * The answer is the state now. **A refusal is not shown**, as the downshift's act is not: it means this
     * window was behind, so it reads again.
     */
    signIn: () => void fleet.startAgentSignIn().then((one) => (one.ok ? set(one.value) : read())),
  };
}
