// Mounts one of the three Cockpit body prototypes in the mock's Cockpit, by the walk that is playing or
// `?direction=`. The mock's page is the only caller, so no prototype reaches the production bundle.
// proto/cockpit-three-directions.

import type { ComponentType } from "react";

import { cockpitBody } from "../../cockpit/body-seam";
import type { CockpitProps } from "../../cockpit/body-seam";
import { appSeam } from "../../cockpit/app-seam";
import { AppBody, AppChrome } from "./App";
import { forgetUi } from "./shared";
import { forgetLocal } from "./ship";
import { Strips } from "./Strips";
import { Wall } from "./Wall";

const BY_DIRECTION: Record<string, ComponentType<CockpitProps>> = {
  strips: Strips,
  wall: Wall,
  app: AppBody,
};

/** The walk that plays each direction, so `?walk=` alone chooses it. */
const BY_WALK: Record<string, string> = {
  cockpitDirectionStrips: "strips",
  cockpitDirectionWall: "wall",
  cockpitDirectionMap: "map",
  cockpitDirectionApp: "app",
};

/** Drafts a prototype keeps in a text box, which a walk must start without. */
const KEPT = "armada:kept:cockpit-proto:";

export function installDirection(walk: string | null, direction: string | null): void {
  const chosen = direction ?? (walk === null ? undefined : BY_WALK[walk]);
  if (chosen === undefined) return;
  for (const key of Object.keys(window.localStorage)) if (key.startsWith(KEPT)) window.localStorage.removeItem(key);
  forgetLocal();
  forgetUi();
  cockpitBody.current = BY_DIRECTION[chosen];
  // The app model also takes the shell: a bar over every page, the calls over all of them, the Work group off the rail.
  appSeam.current = chosen === "app" ? AppChrome : undefined;
  appSeam.hideWork = chosen === "app";
}
