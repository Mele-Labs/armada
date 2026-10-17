// The whole ship, plus Retros, Doctor's probes and a Studio on the same Fleet, so one walk, `restyle-review`, can show the
// Cockpit, its hover cards, the System pages, Retros and Studios in the new look without switching scenarios.

import type { Scenario } from "../moment";
import { pagesLook } from "./pages-look";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

export const restyleReview: Scenario = {
  ...ship,
  name: "restyle-review",
  says: "The whole ship at rest, with Retros, Doctor's probes and a Studio on the same Fleet",
  reads: { ...pagesLook.reads, ...ship.reads },
  studios: pagesLook.studios,
  retros: pagesLook.retros,
  lessons: pagesLook.lessons,
  state: { ...ship.state, health: pagesLook.state.health },
  behaves: (fleet) => ({ ...ship.behaves?.(fleet), ...pagesLook.behaves?.(fleet) }),
};
