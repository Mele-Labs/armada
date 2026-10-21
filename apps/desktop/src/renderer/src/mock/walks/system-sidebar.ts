// System as one sidebar: Kit, Fleet and Bridge in one list, an item's detail beside its kind, a plugin's
// items one press away, Kit's servers in one list, and the mods under Theme. Over `system-sidebar`.

import { button, inside, place, region, role, systemPage, walk } from "../walk";

export const systemSidebar = walk("system-sidebar", [
  { press: place("System"), say: "System" },
  { press: systemPage("Skills"), say: "Skills" },
  { press: inside(region("Skills"), button(/^frontend-design/)), say: "A skill, and its detail" },
  { press: systemPage("Plugins"), say: "Plugins" },
  { press: inside(region("Plugins"), button(/^armada/)), say: "A plugin, and what it brings" },
  { press: inside(role("article", "armada"), button(/^reading-a-brief/)), say: "One of its skills" },
  { press: systemPage("Servers"), say: "Servers" },
  { press: inside(region("Servers"), button(/^tracker/)), say: "A Kit server" },
  { press: systemPage("Models"), say: "Models" },
  { look: region("Models"), say: "The models offered" },
  { press: systemPage("Theme"), say: "Theme, with its mods under it" },
]);
