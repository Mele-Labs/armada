// Mods that travel, over `mods-travel`: Mods turned on in What syncs, Promote opening a pull request and the Job that
// holds it, a mod Armada ships beside a local one of the same name, the Theme rows and Kit's Mods carrying the same
// marks, the two theme.css of a mod that differs, and a kind left off drawing no marks.

import { button, dialog, inside, region, role, systemPage, toSystemPage, walk } from "../walk";

const MENU = dialog("Menu");
const SYNCS = dialog("What syncs");
const PULL = role("dialog", "Pull request #2841");
const DUSK = role("group", "Dusk", { exact: true });
const KIT_MODS = region("Mods", { exact: true });
const entry = (name: string) => inside(role("navigation", "System"), button(new RegExp(`^${name}( |$)`)));

const modsTravel = walk("mods-travel", [
  // What syncs: one switch per kind, Mods among them.
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Account", { exact: true })), say: "The account page" },
  { press: button("What syncs", { exact: true }), say: "What syncs, on the profile added on purpose" },
  { press: inside(SYNCS, role("switch", "Mods", { exact: true })), say: "Mods, off until turned on" },
  { press: inside(SYNCS, button(/Close panel/)), say: "Close" },

  // Promote opens a pull request and the Job that holds it.
  ...toSystemPage("Theme", "Theme"),
  { press: inside(DUSK, button("Promote Dusk")), say: "Promote Dusk" },
  { press: inside(DUSK, button(/^Pull request #2841/)), say: "The pull request, in the side panel" },
  { key: "Escape", on: PULL, say: "Esc" },
  { press: inside(DUSK, button("Job Promote Dusk")), say: "The Job that holds it" },

  // A mod Armada ships, and a local one of the same name.
  ...toSystemPage("Theme", "Theme again"),
  { hover: role("img", "Ships with Armada", { exact: true }), say: "Harbour ships with Armada, so it has no Promote" },
  { hover: button("Add to the home harbour", { exact: true }), say: "The local Harbour is its own row" },

  // The Theme rows carry Kit's marks and acts.
  { hover: inside(DUSK, button("Differs from the home, dusk", { exact: true })), say: "Dusk differs from the home" },
  { press: button("Add to this machine ember", { exact: true }), say: "Ember is only in the home: add it here" },

  // Kit's Mods: the same marks, the acts in the item's detail, and the two theme.css of the one that differs.
  { press: entry("Mods"), say: "Kit, Mods" },
  { key: "ArrowDown", on: inside(KIT_MODS, button(/^dusk/)), say: "Down to the Harbour Armada ships" },
  { press: inside(KIT_MODS, button(/^dusk/)), say: "Dusk differs from the home" },
  { press: inside(region("Detail"), button("Compare with the home")), say: "Open both versions of Dusk" },
  { look: inside(dialog("dusk"), region("The home", { exact: true })), say: "The home's theme.css, its accent lines lit" },
  { press: inside(dialog("dusk"), button(/^Use the home.s$/)), say: "Use the home's" },

  // A kind left off in What syncs draws no marks.
  { press: systemPage("Kit"), say: "Skills, off in What syncs" },
  { look: region("Skills", { exact: true }), say: "No marks on its rows" },
]);

export { modsTravel as "mods-travel" };
