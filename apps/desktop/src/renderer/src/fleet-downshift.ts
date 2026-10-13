// The Session model's step down over Fleet: the state, read once Fleet is connected and kept current by
// `model_downshift.changed`, which carries it whole, and the four acts that answer it. The mock has its own
// source (`mock/downshift.ts`); this one is the real wire.

import type { ModelDownshiftState } from "@armada/protocol";

import type { ModelDownshiftRead } from "../../shared/api/model-downshift";
import type { BridgeState } from "../../shared/bridge";
import type { Downshift, DownshiftSource } from "./downshift";

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetDownshift = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getModelDownshift: () => Promise<ModelDownshiftRead>;
  acceptDownshift: () => Promise<ModelDownshiftRead>;
  restoreDownshift: () => Promise<ModelDownshiftRead>;
  keepDownshift: () => Promise<ModelDownshiftRead>;
  settleDownshift: () => Promise<ModelDownshiftRead>;
  onModelDownshiftChanged: (on: (state: ModelDownshiftState) => void) => () => void;
};

export function createFleetDownshift(fleet: FleetDownshift): DownshiftSource {
  let downshift: Downshift | undefined;
  let wasConnected = false;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: ModelDownshiftState): void => {
    downshift = to.downshift;
    listeners.forEach((on) => on());
  };
  const read = (): void => void fleet.getModelDownshift().then((one) => one.ok && set(one.value));
  /**
   * An act's answer is the state now. **A refusal is not shown**: Fleet refuses an act made in a phase
   * the state has left (409 `fleet.downshift_wrong_phase`), which means this window was behind, so it reads again.
   */
  const act = (press: () => Promise<ModelDownshiftRead>): void =>
    void press().then((one) => (one.ok ? set(one.value) : read()));
  const connectedIn = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    wasConnected = connected;
  };
  const start = (): void => {
    const offState = fleet.subscribe(connectedIn);
    const offChanged = fleet.onModelDownshiftChanged(set);
    void fleet.state().then(connectedIn);
    stop = () => {
      offState();
      offChanged();
      wasConnected = false;
    };
  };

  return {
    get: () => downshift,
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
    accept: () => act(() => fleet.acceptDownshift()),
    restore: () => act(() => fleet.restoreDownshift()),
    keep: () => act(() => fleet.keepDownshift()),
    settle: () => act(() => fleet.settleDownshift()),
  };
}
