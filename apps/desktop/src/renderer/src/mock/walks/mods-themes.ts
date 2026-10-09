// Armada Mods, tier 1: themes, mock only. Switching a theme re-skins the window; a Session writes a
// theme into the mod folder and it joins the picker.

import { button, tab, text, walk } from "../walk";

export const modsThemes = walk("mods-themes", [
  { press: button("Light", { exact: true }), say: "Light re-skins the window" },
  { press: button("Nord"), say: "A theme a mod ships, with its own mark" },
  { press: tab("Mods"), say: "Installed mods" },
  { press: button("Promote nord"), say: "Promote lands on a branch" },
  { look: text("Branch mods/nord pushed. Pull request open."), say: "The outcome is a fact" },
  { press: tab("Session"), say: "A Session in the owner's hands" },
  { press: button("Send"), say: "He asks for a theme" },
  { look: text("Dusk is in Theme."), say: "mod.toml and theme.css are written" },
  { press: tab("Theme"), say: "Back to the picker" },
  { press: button("Dusk"), say: "Dusk is on" },
]);
