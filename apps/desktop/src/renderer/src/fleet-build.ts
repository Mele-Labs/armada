// The build Fleet runs on, as the Fleet panel draws it: main or the preview,
// where it stands against main, and the act on the preview.
//
// **Held in a context, not in `BridgeState`.** Fleet does not serve the build or
// its position yet, so nothing reaches here from main; the mock provides a
// fixture and a real window provides none, which draws no build section.

import { createContext, useContext } from "react";
import type { FleetBuild } from "@armada/components";

const Held = createContext<FleetBuild | undefined>(undefined);

export const FleetBuildFrom = Held.Provider;

export function useFleetBuild(): FleetBuild | undefined {
  return useContext(Held);
}
