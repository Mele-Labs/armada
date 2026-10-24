// A right press on a Cockpit panel offers Mod, and Mod starts a Job for modding Armada carrying what was pointed at. The Job sits
// under Mods in the Jobs list, and its right column lists the mods it made, each with a switch. Over the `mod-job` scenario.

import { deckRow, inside, region, role, text, walk } from "../walk";

const made = region("Mods made", { exact: true });

const modJob = walk("mod-job", [
  { context: region("Activity", { exact: true }), say: "A right press on a panel of the Cockpit" },
  { look: role("menuitem", "Mod"), say: "Mod is in the menu" },
  { press: role("menuitem", "Mod"), say: "Mod starts a Job, titled after what was pointed at" },
  { look: text(/Pointed at:/), say: "Its first message says what was pointed at" },
  { look: inside(region("Jobs", { exact: true }), role("heading", "Mods")), say: "The Job sits under Mods in the list" },
  { look: deckRow(/Activity/), say: "Beside the Jobs of the repository, not among them" },
  { look: inside(made, role("switch", "Dusk")), say: "The mod it made, on, and in force as it was saved" },
  { press: inside(made, role("switch", "Dusk")), say: "Off undoes it" },
]);

export { modJob as "mod-job" };
