// The whole ship, plus Retros, Doctor's probes and Studios kept in two repositories on the same Fleet, so one walk,
// `restyle-review`, can show the Cockpit, its hover cards, the System pages, Retros and Studios in the new look without
// switching scenarios.

import type { Studio } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Scenario } from "../moment";
import { studying } from "../studios-fake";
import { pagesLook } from "./pages-look";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

/** A Studio kept against `manifest`, untouched since `touched`. */
const kept = (id: string, manifest: string, name: string, touched: string): Studio => ({
  id,
  manifest_id: manifest,
  name,
  created_at: touched,
  touched_at: touched,
  nodes: [],
  edges: [],
});

// Armada keeps two and Pocket one; the repositories the ship serves beyond those keep none.
const studied = studying([
  kept("01STUDIOLEGEND0000000000000", repository().manifest!.id, "The Board's legend", "2026-10-15T09:00:00Z"),
  kept("01STUDIOCACHE00000000000000", repository().manifest!.id, "Cache the manifest read", "2026-10-14T16:30:00Z"),
  kept("01STUDIOPAIRING000000000000", "pocket", "Pairing code on the phone", "2026-10-13T11:15:00Z"),
]).scenario;

export const restyleReview: Scenario = {
  ...ship,
  name: "restyle-review",
  says: "The whole ship at rest, with Retros, Doctor's probes and Studios kept in two repositories on the same Fleet",
  studios: studied.studios,
  retros: pagesLook.retros,
  lessons: pagesLook.lessons,
  state: { ...ship.state, health: pagesLook.state.health },
  behaves: (fleet) => ({ ...ship.behaves?.(fleet), ...pagesLook.behaves?.(fleet), ...studied.behaves?.(fleet) }),
};
