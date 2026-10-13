// The write clashes over Fleet. `GET /clashes` is poll-only: Fleet pushes no event for a clash, and
// it finds one 30 seconds after the writes that made it. So this reads when Fleet connects, again
// `SETTLE_MS` after the board changes (the events the rows already refresh on arrive as state), and
// again `SETTLE_MS` after any read that found a clash standing, so a cleared one goes. At most one
// read is pending, so a busy board costs one read per `SETTLE_MS` and an idle one costs none.

import type { WriteClash } from "@armada/protocol";

import type { WriteClashesRead } from "../../shared/api/clashes";
import type { BridgeState } from "../../shared/bridge";

/** A little past the 30 seconds Fleet waits before it asks git. */
export const SETTLE_MS = 35_000;

export type ClashesSource = {
  get(): readonly WriteClash[];
  subscribe(on: () => void): () => void;
};

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetClashes = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getWriteClashes: () => Promise<WriteClashesRead>;
};

/** Where a pending read waits: `setTimeout` in the app, a hand-turned clock in a test. */
export type Later = (run: () => void, ms: number) => () => void;

const timer: Later = (run, ms) => {
  const id = setTimeout(run, ms);
  return () => clearTimeout(id);
};

const NONE: readonly WriteClash[] = [];

export function createFleetClashes(fleet: FleetClashes, later: Later = timer): ClashesSource {
  let clashes = NONE;
  let wasConnected = false;
  let cancel: (() => void) | null = null;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (to: readonly WriteClash[]): void => {
    if (JSON.stringify(to) === JSON.stringify(clashes)) return;
    clashes = to.length === 0 ? NONE : to;
    listeners.forEach((on) => on());
  };
  const settle = (): void => {
    if (cancel !== null) return;
    cancel = later(() => {
      cancel = null;
      read();
    }, SETTLE_MS);
  };
  const read = (): void => {
    void fleet.getWriteClashes().then((answer) => {
      // A read that failed leaves what stands: a refusal is not news that a clash cleared.
      if (!answer.ok) return;
      set(answer.value);
      if (answer.value.length > 0) settle();
    });
  };
  const changed = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    else if (connected) settle();
    else if (wasConnected) {
      cancel?.();
      cancel = null;
      set(NONE);
    }
    wasConnected = connected;
  };
  const start = (): void => {
    const off = fleet.subscribe(changed);
    void fleet.state().then(changed);
    stop = () => {
      off();
      cancel?.();
      cancel = null;
      wasConnected = false;
    };
  };

  return {
    get: () => clashes,
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
