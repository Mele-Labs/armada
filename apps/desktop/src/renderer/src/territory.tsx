// What the flight deck's map is drawn from, as the window holds it: Fleet's directories for each repository and what each
// slot's holder has changed. `fleet-territory.ts` reads it in the app; a mock scenario hands `AreasFrom` its own.

import { useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import { AreasFrom } from "./cockpit/deck/areas-source";
import { SlotTouchesFrom } from "./cockpit/deck/touches-source";
import type { TerritorySource } from "./fleet-territory";

/** The territory Fleet reports, kept current by `source`. The provider is always there, so a read landing does not remount the app. */
export function TerritoryFromFleet({ source, children }: { source: TerritorySource; children: ReactNode }) {
  const territory = useSyncExternalStore(source.subscribe, source.get);
  return (
    <AreasFrom source={territory.areas}>
      <SlotTouchesFrom touches={territory.touches}>{children}</SlotTouchesFrom>
    </AreasFrom>
  );
}
