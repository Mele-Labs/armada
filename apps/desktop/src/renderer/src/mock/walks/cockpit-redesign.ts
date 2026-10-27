// The Cockpit's density redesign end to end, for review before it lands: icon Places, one-line Job rows with the View menu and the hover
// card, Bays with no glyphs, the deck with no name bands and its keys as tiles, an open Job's head views and Flight control lamps, and the
// Autopilot panel's switch. Over the `flight-control` scenario.

import { button, deckRow, inside, place, region, role, toSystemPage, walk } from "../walk";

const list = region("Jobs", { exact: true });

const cockpitRedesign = walk("flight-control", [
  { hover: place("Studios"), say: "Places are icons. The name is on hover" },
  { look: list, say: "One line per Job: name and state. Hairlines between Jobs, space between repositories" },
  { hover: deckRow("Cache key"), say: "The full hover card on a row" },
  { context: deckRow("Cache key"), say: "Right-click a row for View" },
  { press: role("menuitem", "Active PRs", { exact: true }), say: "Active PRs on, a line of its own under the name" },
  { press: role("menuitem", "Now", { exact: true }), say: "Now on, with Active PRs under it" },
  { key: "Escape", on: role("menu"), say: "Esc closes the menu" },
  { look: region("Bays"), say: "Bays: the number, the Job and its branch. No glyphs, no time" },
  { look: region("Flight controls"), say: "No name bands. The keys are tiles, set in the panel" },
  { look: region("Dispatch controls"), say: "Dispatch is one tile wide" },
  { look: region("Merge queue"), say: "The merge line, with more room around its chips" },
  { press: deckRow("Tune the flight controls"), say: "A Job open. Its name and state are in its own head" },
  { look: button("Artifacts"), say: "Its views are icons in the head: Artifacts, Checks, Subagents, Bays" },
  { press: button("Artifacts"), say: "Artifacts over the thread" },
  { press: button("Artifacts"), say: "Pressed again, back to the thread" },
  { look: region("Flight controls"), say: "Flight control: one row of keys" },
  { press: inside(region("Flight controls"), button("Auto land")), say: "Auto land seats its key and lights its lamp" },
  { hover: inside(region("Flight controls"), button("Edit")), say: "Edit is the pen" },
  { press: deckRow("Draft the release notes"), say: "A Job just begun" },
  { hover: button("Checks"), say: "Checks is off. The tooltip says why" },
  ...toSystemPage("Features", "Features"),
  { look: role("switch", /^Autopilot panel/), say: "The Autopilot panel is off unless this is on" },
  { press: role("switch", /^Autopilot panel/), say: "On" },
  { press: place("Cockpit"), say: "Back to the Cockpit" },
  { press: button("Back", { exact: true }), say: "Close the Job" },
  { look: region("Autopilot"), say: "With no Job open, the Autopilot panel is drawn" },
]);

export { cockpitRedesign as "cockpit-redesign" };
