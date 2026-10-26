// A mod a Mod Job made is drawn in its thread as the card System draws, and only its switch toggles it: the rest of the row opens the mod
// in the side panel, with Change it, which starts an empty Mod Job aimed at what the mod is. Over the `mod-job` scenario.
// The switch is the slide toggle with cut corners; `?switch=rail` draws its track as a rail.

import { button, dialog, inside, region, role, systemPage, text, place, viaPalette, walk } from "../walk";

const thread = region("Thread");
const dusk = role("group", "Dusk");

const modCard = walk("mod-job", [
  ...viaPalette("Create mod", "⌘K, then Create mod"),
  { type: "Make a dusk theme\n", into: role("textbox", "Message"), say: "He describes what he wants and sends it" },
  { look: inside(thread, inside(dusk, role("switch", "Dusk"))), say: "The mod the Job made is in the thread, drawn as System draws it, with its switch" },
  { press: inside(thread, button("Open Dusk")), say: "A press on the row anywhere but the switch opens the mod, and leaves it on" },
  { look: inside(dialog("Mod Dusk"), button("Change it")), say: "Its facts in the side panel, and Change it" },
  { press: inside(dialog("Mod Dusk"), button("Change it")), say: "Change it starts an empty Mod Job" },
  { look: inside(region("What a Mod Job makes"), role("radio", "Colours")), say: "Aimed at Colours" },
  { look: text("Change the Dusk theme:"), say: "The box opens on the change, the caret after it" },
  { press: place("System"), say: "System" },
  { press: systemPage("All"), say: "Mods, all of them" },
  { press: inside(dusk, role("switch", "Dusk")), say: "The switch alone turns Dusk off" },
]);

export { modCard as "mod-card" };
