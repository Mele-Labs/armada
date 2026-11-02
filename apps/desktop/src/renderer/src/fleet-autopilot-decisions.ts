// Autopilot's answers over Fleet: the list, read once Fleet is connected and kept current by
// `autopilot.decisions_changed`, which carries it whole. The mock mounts the same source over its own Fleet.

import type { AutopilotDecided, AutopilotDecisions } from "@armada/protocol";

import type { AutopilotDecisionsActed } from "../../shared/api/autopilot-decisions";
import type { BridgeState } from "../../shared/bridge";
import type { AutopilotDecisionsSource } from "./autopilot-decisions";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetAutopilotDecisions = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getAutopilotDecisions: () => Promise<AutopilotDecisionsActed>;
  overrideAutopilotDecision: (id: string, text: string) => Promise<AutopilotDecisionsActed>;
  reviewAutopilotDecision: (id: string) => Promise<AutopilotDecisionsActed>;
  onAutopilotDecisionsChanged: (on: (decisions: AutopilotDecisions) => void) => () => void;
};

const NOTHING: readonly AutopilotDecided[] = [];

export function createFleetAutopilotDecisions(fleet: FleetAutopilotDecisions): AutopilotDecisionsSource {
  let state = NOTHING;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  // A correction that is still out with Fleet. Fleet marks its row reviewed itself, so a review sent
  // beside it would race it and could land first.
  const correcting = new Map<string, Promise<boolean>>();
  const set = (to: AutopilotDecisions): void => {
    state = to.to_review;
    listeners.forEach((on) => on());
  };
  const took = (acted: AutopilotDecisionsActed): boolean => {
    if (acted.ok) set(acted.value);
    return acted.ok;
  };
  const read = (): void => void fleet.getAutopilotDecisions().then(took);
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offDecisions = fleet.onAutopilotDecisionsChanged(set);
    void fleet.state().then(connectedIn);
    stop = () => {
      offState();
      offDecisions();
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
    override: (id, text) => {
      const out = fleet.overrideAutopilotDecision(id, text).then(took);
      correcting.set(id, out);
      void out.finally(() => correcting.get(id) === out && correcting.delete(id));
    },
    // After a correction the row is already gone, and one Fleet refused stays for the owner.
    review: (id) =>
      void (correcting.get(id) ?? Promise.resolve(true)).then((taken) => {
        if (taken && state.some((one) => one.id === id)) void fleet.reviewAutopilotDecision(id).then(took);
      }),
  };
}
