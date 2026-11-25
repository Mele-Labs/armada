// Refresh preview from the phone: Fleet's `change_fleet_build` onto the preview, adopting the Jobs that are working. Where
// another Mac is linked, that Mac merges both Macs' ready offers and asks the other to build the same commit.

import { call } from "./client";
import type { Machine } from "./machines";

export const refreshPreview = async (machine: Machine): Promise<void> => {
  await call(machine, "/api/fleet/build/change", {
    method: "POST",
    body: {
      build: "preview",
      adopt: true,
    },
  });
};
