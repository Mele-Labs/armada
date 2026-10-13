// Sleep mode over Fleet: the switch and the night, read once Fleet is connected and kept current by
// `sleep.changed`, which carries the night whole. The mock has its own source; this one is the real wire.

import type { SleepState as SleepWire } from "@armada/protocol";

import type { SleepActed } from "../../shared/api/sleep";
import type { BridgeState } from "../../shared/bridge";
import type { SleepSource, SleepState } from "./sleep";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetSleep = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getSleep: () => Promise<SleepActed>;
  setSleep: (on: boolean) => Promise<SleepActed>;
  overrideSleep: (id: string, text: string) => Promise<SleepActed>;
  reviewSleep: (id: string) => Promise<SleepActed>;
  onSleepChanged: (on: (state: SleepWire) => void) => () => void;
};

const NOTHING: SleepState = { on: false, decided: [], blocked: [], landed: [], walks: [] };

/** The wire's night as Bridge draws it: a decision's `session_id` is the Session it opens. */
const nightOf = (wire: SleepWire): SleepState => ({
  ...wire,
  decided: wire.decided.map(({ session_id, ...one }) => (session_id === undefined ? one : { ...one, sessionId: session_id })),
});

export function createFleetSleep(fleet: FleetSleep): SleepSource {
  let state = NOTHING;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  // A correction that is still out with Fleet. Fleet marks its row reviewed itself, so a review sent
  // beside it would race it and could land first.
  const correcting = new Map<string, Promise<boolean>>();
  const set = (to: SleepWire): void => {
    state = nightOf(to);
    listeners.forEach((on) => on());
  };
  const took = (acted: SleepActed): boolean => {
    if (acted.ok) set(acted.value);
    return acted.ok;
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
    override: (id, text) => {
      const out = fleet.overrideSleep(id, text).then(took);
      correcting.set(id, out);
      void out.finally(() => correcting.get(id) === out && correcting.delete(id));
    },
    // After a correction the row is already gone, and one Fleet refused stays for the owner.
    review: (id) =>
      void (correcting.get(id) ?? Promise.resolve(true)).then((taken) => {
        if (taken && state.decided.some((one) => one.id === id)) void fleet.reviewSleep(id).then(took);
      }),
  };
}
