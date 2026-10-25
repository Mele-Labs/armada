// Create mod, in the palette and on the Mods page of System: it starts an empty Mod Job at once and opens it on the Cockpit, the box ready for his words.
// The empty thread lists what a Mod Job can make. His first message names the Job, which sits under Mods. Over the `mod-job` scenario.

import { button, inside, region, role, systemPage, text, place, viaPalette, walk } from "../walk";

const words = "Make the Cockpit tabs wider";
const message = role("textbox", "Message");

const createMod = walk("mod-job", [
  ...viaPalette("Create mod", "⌘K, then Create mod"),
  { look: inside(region("What a Mod Job makes"), text("Colours")), say: "An empty Mod Job opens, listing what it can make" },
  { type: `${words}\n`, into: message, say: "The box has the caret: he types what he wants and sends it" },
  { look: inside(region("Jobs", { exact: true }), button(`Mod ${words}`)), say: "His first line names the Job" },
  { look: inside(region("Jobs", { exact: true }), role("heading", "Mods")), say: "It sits under Mods in the list" },
  { press: place("System"), say: "System" },
  { press: systemPage("All"), say: "Mods, all of them" },
  { press: button("Create mod"), say: "Create mod in the header of the list opens another empty Mod Job" },
  { look: inside(region("What a Mod Job makes"), text("Anything else")), say: "Empty again, with the list" },
]);

export { createMod as "create-mod" };
