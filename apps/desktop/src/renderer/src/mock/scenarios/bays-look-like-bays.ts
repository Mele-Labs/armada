// The Cockpit's Bays panel with a bay in every state to look at: held by a Session that is working, free and warm, free
// and cold, stranded with what it left, and closed. The cockpit ship's pool, plus a warm free bay it had none of. The claims are in
// `walks/bays-look-like-bays.ts`.

import type { WorktreeSlot } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

function build(): Scenario {
  const armada = ship.held?.slots?.find((one) => one.slot === 2)?.manifest_id ?? "armada";
  const warm: WorktreeSlot = {
    manifest_id: armada,
    slot: 7,
    path: "/Users/user/armada/.armada/slots/slot-7",
    base: "main",
    warm: true,
    behind: 0,
    held: { state: "free" },
  };
  return {
    ...ship,
    name: "bays-look-like-bays",
    says: "The Cockpit's Bays panel drawn as bays, a bay in each state: held, free warm and cold, stranded, closed",
    held: { slots: [...(ship.held?.slots ?? []), warm] },
  };
}

export const s262BaysLookLikeBays: Scenario = build();
