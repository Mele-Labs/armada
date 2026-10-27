// The fixes the owner asked for on 26 Oct 2026, after the Cockpit's density pass: the notch's places as icons with small names and a
// hairline before the search, ⌘ digits in the order the places are drawn, room between the bays, Dispatch a raised key and not a pressed one, an open Job's side panels standing apart and folding by their bands again, a composer
// two lines tall. Over the `flight-control` scenario; the Reviews page's list band is `reviews-band`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const deckSpacingFixes = walk("flight-control", [
  { look: role("navigation", "Places"), say: "Each place is its icon and its name, smaller. A hairline before the search" },
  { key: "Meta+2", on: role("navigation", "Places"), say: "⌘2 is the second place drawn: Studios" },
  { key: "Meta+1", on: role("navigation", "Places"), say: "⌘1, back to the Cockpit" },
  { look: region("Bays"), say: "Room between the bays" },
  { look: region("Dispatch controls"), say: "Dispatch is a raised key like the others, its glyph in the accent. Only a pressed key is filled" },
  { press: deckRow("Tune the flight controls"), say: "A Job open" },
  { look: region("Field", { exact: true }), say: "Each side panel stands apart, folded or open" },
  { press: inside(region("Field", { exact: true }), button(/\bField\b/)), say: "Field folds by its band" },
  { press: inside(region("Field", { exact: true }), button(/\bField\b/)), say: "And opens again" },
  { press: inside(region("Now", { exact: true }), button(/\bNow\b/)), say: "Now folds by its band too" },
  { look: role("textbox", "Message"), say: "The composer is two lines tall" },
]);

export { deckSpacingFixes as "deck-spacing-fixes" };
