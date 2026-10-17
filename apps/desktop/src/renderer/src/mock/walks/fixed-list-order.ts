// The Cockpit's Work list keeps the order it was given: opening a Session whose call is amber does not move its row, a row
// is dragged to a new place, and ⌥↓ moves the one under the cursor down by one. Over `cockpit-ship-flightdeck`.

import { deckRow, walk } from "../walk";

export const fixedListOrder = walk("cockpit-ship-flightdeck", [
  { look: deckRow("Cache key"), say: "Cache key has a question open, so its row is amber and first" },
  { press: deckRow("Cache key"), say: "Opened, its question is seen and the row goes quiet. It stays first" },
  { drag: deckRow("Changelog script"), by: { x: 0, y: -150 }, say: "Drag a row to move it. A line shows where it will be put down, and it stays among the rows of its own repository" },
  { key: "Alt+ArrowDown", on: deckRow("Migration notes"), say: "⌥↓ moves the row under the cursor down one, ⌥↑ up one" },
]);
