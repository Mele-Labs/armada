// Armada Mods, tier 1: themes, mock only. Theme in Settings re-skins the window; Mods lists what is
// installed; a Session writes a theme into the mod folder and it joins both.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";

const { message, thread, rail } = kit(false);

export const modsThemes = walk("mods-themes", [
  { press: rail("Settings"), say: "Settings" },
  { look: role("radio", "Dark"), say: "Dark is in force" },
  { press: role("radio", "Light"), say: "Light re-skins the window" },
  { look: role("img", "From the nord mod"), say: "A theme a mod ships carries the puzzle glyph" },
  { press: role("radio", "Nord"), say: "Nord" },
  { press: rail("Mods"), say: "Mods, under Machine" },
  { look: role("img", "Theme"), say: "Each row names its kind" },
  { press: button("Promote Nord"), say: "Promote" },
  { look: text("Branch mods/nord pushed. Pull request open."), say: "The outcome is a fact" },
  toSessions,
  { press: inside(region("Sessions"), button(/A theme/)), say: "A Session Bridge hosts" },
  { type: "Make a dusk theme: purple-grey ground, a warm accent.\n", into: message, say: "The request" },
  { look: inside(thread, text(/theme\.css/)), say: "theme.css written into the mod folder" },
  { press: rail("Settings"), say: "Settings" },
  { press: role("radio", "Dusk"), say: "Dusk is in the picker" },
  { press: rail("Mods"), say: "And in Mods" },
  { look: role("group", "Dusk"), say: "Dusk" },
]);
