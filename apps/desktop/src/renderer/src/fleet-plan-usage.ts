// The plan's windows over Fleet: read once Fleet is connected and kept current by `plan_usage.changed`,
// which carries the reading whole. Read at once as well, since main answers from the picture it kept on disk
// while Fleet has not answered. The mock's Fleet answers the same two calls from its scenario.

import type { PlanUsage } from "@armada/protocol";

import type { PlanUsageRead } from "../../shared/api/plan-usage";
import type { BridgeState } from "../../shared/bridge";
import type { PlanUsageSource } from "./plan-usage";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetPlanUsage = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getPlanUsage: () => Promise<PlanUsageRead>;
  onPlanUsageChanged: (on: (usage: PlanUsage) => void) => () => void;
};

export function createFleetPlanUsage(fleet: FleetPlanUsage): PlanUsageSource {
  let usage: PlanUsage | undefined;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: PlanUsage): void => {
    usage = to;
    listeners.forEach((on) => on());
  };
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) void fleet.getPlanUsage().then((read) => read.ok && set(read.value));
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offUsage = fleet.onPlanUsageChanged(set);
    void fleet.state().then((facts) => {
      if (facts.connection.state !== "connected") void fleet.getPlanUsage().then((read) => read.ok && set(read.value));
      connectedIn(facts);
    });
    stop = () => {
      offState();
      offUsage();
      wasConnected = false;
    };
  };

  return {
    get: () => usage,
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
  };
}
