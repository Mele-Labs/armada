// The directories each repository is drawn in and what each slot's holder has changed, over Fleet: read when Fleet
// connects, again `SETTLE_MS` after the board changes, and again `SETTLE_MS` after any read that found a holder with
// changes, so a holder that stops goes. At most one read is pending. A read Fleet refused leaves what stands, each half
// on its own; a Fleet that has gone leaves nothing standing, and one that answers none draws none.

import type { RepositoryAreas, SlotTouches } from "@armada/protocol";

import type { RepositoryAreasRead, SlotTouchesRead } from "../../shared/api/territory";
import type { BridgeState } from "../../shared/bridge";
import { areasFromFleet, noAreas, type AreaSource } from "./cockpit/deck/areas";
import { touchesOfSlots, type Touch } from "./cockpit/deck/touches";

/** The 35 seconds the write clashes settle for, so the deck's map and its marks move together. */
export const SETTLE_MS = 35_000;

/** What the deck reads of Fleet's territory. A new object whenever either half changed. */
export type Territory = { areas: AreaSource; touches: ReadonlyMap<string, readonly Touch[]> };

export type TerritorySource = {
  get(): Territory;
  subscribe(on: () => void): () => void;
};

/** What Bridge reaches Fleet through: `window.armada` in the app, a fake in a test. */
export type FleetTerritory = {
  state: () => Promise<Pick<BridgeState, "connection">>;
  subscribe: (on: (state: Pick<BridgeState, "connection">) => void) => () => void;
  getRepositoryAreas: () => Promise<RepositoryAreasRead>;
  getSlotTouches: () => Promise<SlotTouchesRead>;
};

/** Where a pending read waits: `setTimeout` in the app, a hand-turned clock in a test. */
export type Later = (run: () => void, ms: number) => () => void;

const timer: Later = (run, ms) => {
  const id = setTimeout(run, ms);
  return () => clearTimeout(id);
};

const NO_AREAS: RepositoryAreas = { repositories: [] };
const NO_TOUCHES: SlotTouches = { repositories: [] };
const NONE: Territory = { areas: noAreas, touches: new Map() };

export function createFleetTerritory(fleet: FleetTerritory, later: Later = timer): TerritorySource {
  let territory = NONE;
  let areas = NO_AREAS;
  let touches = NO_TOUCHES;
  let wasConnected = false;
  let cancel: (() => void) | null = null;
  const listeners = new Set<() => void>();
  let stop: (() => void) | null = null;

  const set = (toAreas: RepositoryAreas, toTouches: SlotTouches): void => {
    const sameAreas = JSON.stringify(toAreas) === JSON.stringify(areas);
    const sameTouches = JSON.stringify(toTouches) === JSON.stringify(touches);
    if (sameAreas && sameTouches) return;
    areas = toAreas;
    touches = toTouches;
    territory = areas.repositories.length === 0 && touches.repositories.length === 0 ? NONE : { areas: areasFromFleet(areas), touches: touchesOfSlots(touches) };
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
    void Promise.all([fleet.getRepositoryAreas(), fleet.getSlotTouches()]).then(([asked, found]) => {
      // A half Fleet refused stands as it was: a refusal is not news that a directory or a change is gone.
      set(asked.ok ? asked.value : areas, found.ok ? found.value : touches);
      if (found.ok && found.value.repositories.some((one) => one.holders.length > 0)) settle();
    });
  };
  const changed = (facts: Pick<BridgeState, "connection">): void => {
    const connected = facts.connection.state === "connected";
    if (connected && !wasConnected) read();
    else if (connected) settle();
    else if (wasConnected) {
      cancel?.();
      cancel = null;
      set(NO_AREAS, NO_TOUCHES);
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
    get: () => territory,
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
