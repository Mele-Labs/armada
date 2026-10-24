// Create mod, in the palette and on the Mods page of System: a box with one field and Start, which starts a Mod Job from a description alone.
// The Job opens on the Cockpit with his words as its first message and sits under Mods. Over the `mod-job` scenario.

import { button, dialog, inside, region, role, systemPage, text, place, walk } from "../walk";

const palette = dialog("Command palette");
const box = dialog("Create mod");
const words = "Make the Cockpit tabs wider";

const createMod = walk("mod-job", [
  { key: "Meta+k", on: role("navigation", "Places"), say: "⌘K opens the palette" },
  { type: "Create mod", into: inside(palette, role("combobox")), say: "Create mod is found by its name" },
  { press: role("option", /^Create mod/), say: "Choose Create mod" },
  { look: inside(box, button("Start")), say: "One field and Start, which is off while the field is empty" },
  { type: words, into: inside(box, role("textbox", "Description")), say: "What he wants, in his own words" },
  { press: inside(box, button("Start")), say: "Start, or ⌘↵" },
  { look: text(new RegExp(`^${words}$`)), say: "The Job opens with his words as its first message" },
  { look: inside(region("Jobs", { exact: true }), role("heading", "Mods")), say: "It sits under Mods in the list" },
  { press: place("System"), say: "System" },
  { press: systemPage("All"), say: "Mods, all of them" },
  { look: button("Create mod"), say: "Create mod is in the header of the list, and opens the same box" },
]);

export { createMod as "create-mod" };
