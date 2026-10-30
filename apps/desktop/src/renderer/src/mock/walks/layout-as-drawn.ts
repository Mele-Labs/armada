// Settings → Layout lists what the Cockpit and an open Job draw today, and each place once. Activity switched off leaves the deck and
// Bays takes the right column; Open switched off leaves an open Job with Done and Now. A layout mod says the same by id. Over `compact-session-view`.

import { deckRow, inside, place, region, role, toSystemPage, walk } from "../walk";

export const layoutAsDrawn = walk("compact-session-view", [
  ...toSystemPage("Layout", "Layout, from System"),
  { look: role("group", "Cockpit panels"), say: "The Cockpit's panels as drawn. The Jobs list always stays" },
  { look: role("group", "Job panels"), say: "An open Job's panels, in the order they stand" },
  { look: role("group", "Places"), say: "Each place once" },
  { press: role("switch", "Activity"), say: "Activity off" },
  { press: inside(role("group", "Job panels"), role("switch", "Open")), say: "Open off" },
  { press: place("Cockpit"), say: "The Cockpit" },
  { look: region("Bays", { exact: true }), say: "No Activity under Bays" },
  { press: deckRow("Retry the slot lease"), say: "A Job" },
  { look: region("Build", { exact: true }), say: "The right column stands with the Job's current step. Open is gone" },
]);
