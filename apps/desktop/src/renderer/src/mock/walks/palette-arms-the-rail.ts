// The palette's Add rows arm their kind on a Studio's rail, Add a zone among them, and its Run row
// puts focus in the rail's Run menu — the owner, 2 Oct 2026. `studio-palette.test.tsx` holds the
// claims, `R` and the read-only reasons with them.

import { button, inside, role, walk } from "../walk";

const rail = role("group", "What you can put on this Studio");

export const paletteArmsTheRail = walk("studios", [
  { press: button("Studios", { exact: true }), say: "This repository's Studios" },
  { press: role("cell", "The Board's legend", { exact: true }), say: "Opened from the list, it is read-only" },
  { press: button("Continue"), say: "Continue makes the Studio editable" },
  { press: button("Search, or describe work"), say: "The palette, as ⌘K opens it" },
  { press: role("option", /^Add a note/), say: "Choose Add a note" },
  { look: inside(rail, button("Add a Note", { exact: true })), say: "Note is armed on the rail, as N arms it" },
  { press: button("Search, or describe work"), say: "The palette again" },
  { press: role("option", /^Add a zone/), say: "Choose Add a zone, which did nothing before" },
  { look: inside(rail, button("Add a Zone", { exact: true })), say: "Zone is armed in Note's place" },
  { press: button("Search, or describe work"), say: "The palette again" },
  { press: role("option", /^Run\b/), say: "Choose Run" },
  { look: role("menu"), say: "The Run menu opens with focus on its first item, so ↓ and Enter work" },
]);
