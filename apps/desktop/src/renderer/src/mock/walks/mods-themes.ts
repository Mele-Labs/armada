// Armada Mods, tier 1: themes. Theme in Settings is one field, grouped and searchable, over the
// catalogue Bridge ships; Mods lists what is installed; a Session writes a theme into the mod folder
// and it joins both.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { message, thread, rail } = kit(false);
const theme = role("combobox", "Theme");

export const modsThemes = walk("mods-themes", [
  { press: rail("Settings"), say: "Settings" },
  { look: theme, say: "Dark is in force" },
  { type: "light", into: theme, say: "Typing narrows the list" },
  { press: role("option", "Light", { exact: true }), say: "Light re-skins the window" },
  { type: "nord", into: theme, say: "A theme Bridge ships" },
  { press: role("option", "Nord", { exact: true }), say: "Nord" },
  { type: "latte", into: theme, say: "Light themes are in the list too" },
  { press: role("option", "Catppuccin Latte"), say: "Catppuccin Latte" },
  { press: rail("Mods"), say: "Mods, under Machine" },
  toSessions,
  { press: inside(region("Sessions"), button(/A theme/)), say: "A Session Bridge hosts" },
  { type: "Make a dusk theme: purple-grey ground, a warm accent.\n", into: message, say: "The request" },
  { look: inside(thread, text(/theme\.css/)), say: "theme.css written into the mod folder" },
  { press: rail("Settings"), say: "Settings" },
  { type: "dusk", into: theme, say: "Dusk" },
  { press: role("option", "Dusk"), say: "Dusk is in the list, under From mods" },
  { press: rail("Mods"), say: "And in Mods" },
  { look: role("img", "Theme"), say: "Each row names its kind" },
  { look: role("group", "Dusk"), say: "Dusk" },
  { press: button("Promote Dusk"), say: "Promote" },
  { look: text("Branch mods/dusk pushed. Pull request open."), say: "The outcome is a fact" },
]);
