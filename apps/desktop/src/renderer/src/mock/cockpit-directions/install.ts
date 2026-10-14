// Mounts one of the three Cockpit body prototypes in the mock's Cockpit, by the walk that is playing or
// `?direction=`. The mock's page is the only caller, so no prototype reaches the production bundle.
// proto/cockpit-three-directions.

import type { ComponentType } from "react";

import { cockpitBody } from "../../cockpit/body-seam";
import type { CockpitProps } from "../../cockpit/body-seam";
import { forgetLocal } from "./ship";
import { Strips } from "./Strips";

const BY_DIRECTION: Record<string, ComponentType<CockpitProps>> = {
  strips: Strips,
};

/** The walk that plays each direction, so `?walk=` alone chooses it. */
const BY_WALK: Record<string, string> = {
  cockpitDirectionStrips: "strips",
  cockpitDirectionWall: "wall",
  cockpitDirectionMap: "map",
};

/** Drafts a prototype keeps in a text box, which a walk must start without. */
const KEPT = "armada:kept:cockpit-proto:";

export function installDirection(walk: string | null, direction: string | null): void {
  const chosen = direction ?? (walk === null ? undefined : BY_WALK[walk]);
  if (chosen === undefined) return;
  for (const key of Object.keys(window.localStorage)) if (key.startsWith(KEPT)) window.localStorage.removeItem(key);
  forgetLocal();
  cockpitBody.current = BY_DIRECTION[chosen];
}
