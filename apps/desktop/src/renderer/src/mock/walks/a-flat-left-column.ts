// The left column drawn flat: Navigation and Fleet are a solid fill under
// a thin border, while the Dashboard's call keeps the card treatment.
// The owner's decision of 7 Oct 2026; `left-column-flat.test.tsx` holds the claims.

import { region, role, text, walk } from "../walk";

export const aFlatLeftColumn = walk("every-state", [
  { look: role("navigation", "Work", { exact: true }), say: "Navigation, Work: flat, no bevel, no shadow" },
  { look: role("navigation", "Machine", { exact: true }), say: "Navigation, Machine: the same" },
  { look: text("Fleet"), say: "Fleet: a solid fill and a thin border" },
  { look: region(/^[A-Z][A-Za-z ]+: /), say: "The Dashboard's call, beside them, keeps its depth" },
]);
