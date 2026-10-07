// The left column drawn flat: Navigation, Stats and Fleet are a solid fill under
// a thin border, while Overview's cards on the canvas keep the card treatment.
// The owner's decision of 7 Oct 2026; `left-column-flat.test.tsx` holds the claims.

import { role, text, walk } from "../walk";

export const aFlatLeftColumn = walk("every-state", [
  { look: role("navigation", "Work", { exact: true }), say: "Navigation, Work: flat, no bevel, no shadow" },
  { look: role("navigation", "Machine", { exact: true }), say: "Navigation, Machine: the same" },
  { look: text("Stats"), say: "Stats: a solid fill and a thin border" },
  { look: text("Fleet"), say: "Fleet: the same" },
  { look: text("Awaiting approval"), say: "An Overview card beside them keeps its depth" },
]);
