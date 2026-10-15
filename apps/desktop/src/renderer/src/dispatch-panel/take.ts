// Which take of the quick Dispatch the mock is playing. The walks name them (`quick-dispatch-centre`,
// `quick-dispatch-deck-box`), or `?placement=` and `?deck=` do; nothing in the real window chooses, and it takes the first
// of each. Where the panel sits, and what the Cockpit's Dispatch panel holds.

import type { Placement } from "./DispatchPanel";

/** What the Cockpit's Dispatch panel holds: one big tile, or a one-line request box. */
export type DeckDispatch = "button" | "box";

/** The request box on the Cockpit's deck, which `n` gives the caret to while it is drawn. */
export const DECK_BOX = ".deck-dispatch__field";

const query = () => new URLSearchParams(window.location.search);
const walking = () => query().get("walk") ?? "";

export function placementOfWindow(): Placement {
  return query().get("placement") === "centre" || walking().endsWith("-centre") || walking().includes("-deck-") ? "centre" : "title";
}

export function deckDispatchOfWindow(): DeckDispatch {
  return query().get("deck") === "box" || walking().endsWith("-box") ? "box" : "button";
}
