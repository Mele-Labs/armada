// The Cockpit tab's one body: the flight deck. `cockpit/deck/` holds the parts.

import { DeckBody } from "./cockpit/deck/Deck";
import type { CockpitProps } from "./cockpit/deck/props";

export function CommandCentral(props: CockpitProps) {
  return <DeckBody {...props} />;
}
