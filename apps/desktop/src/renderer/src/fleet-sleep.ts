// Sleep mode over Fleet: the switch and the night, read once Fleet is connected and kept current by
// `sleep.changed`, which carries the night whole. The mock has its own source; this one is the real wire.

import type { SleepState } from "@armada/protocol";

import type { SleepActed } from "../../shared/api/sleep";
import type { BridgeState } from "../../shared/bridge";
import type { SleepSource } from "./sleep";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetSleep = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getSleep: () => Promise<SleepActed>;
  setSleep: (on: boolean) => Promise<SleepActed>;
  overrideSleep: (id: string, text: string) => Promise<SleepActed>;
  onSleepChanged: (on: (state: SleepState) => void) => () => void;
};

const NOTHING: SleepState = { on: false, decided: [], blocked: [], landed: [], walks: [] };

export function createFleetSleep(fleet: FleetSleep): SleepSource {
  let state = NOTHING;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: SleepState): void => {
    state = to;
    listeners.forEach((on) => on());
  };
  const took = (acted: SleepActed): void => {
    if (acted.ok) set(acted.value);
  };
  const read = (): void => void fleet.getSleep().then(took);
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offSleep = fleet.onSleepChanged(set);
    void fleet.state().then(connectedIn);
    stop = () => {
      offState();
      offSleep();
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
    toggle: () => void fleet.setSleep(!state.on).then(took),
    override: (id, text) => void fleet.overrideSleep(id, text).then(took),
    // Reviewed is not on the wire yet.
    review: () => undefined,
  };
}
