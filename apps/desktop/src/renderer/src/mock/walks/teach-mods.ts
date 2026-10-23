// Mods are split by what they change: Theme, Layout, Words and Fleet. What was taught in `teach-anything` is two of them, one of each
// kind. Then Teach on Bridge itself: a right press on a row that needs him, "make these louder", shown back as a theme tweak with
// every other place the colour is drawn, which survives choosing another theme; and Shift+F10 on the Field panel, "hide this",
// shown back as a layout change. Over the `teach-mods` scenario, which starts with the Jira key and the batched notice taught.

import { toSessions } from "../sessions/walk-kit";
import { ARMADA_TITLE, PAYMENTS_TITLE } from "../sessions/teach";
import { button, deckRow, dialog, inside, place, region, role, systemPage, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const panel = dialog("Teach");
  const instruction = inside(panel, role("textbox", "Instruction"));
  const section = (name: string) => inside(panel, region(name, { exact: true }));
  const menuItem = (name: string) => role("menuitem", name);
  const theme = role("combobox", "Theme");
  const tweak = role("switch", "needs-you-louder");
  const field = region("Field", { exact: true });
  return [
    // Every mod on the machine in one list.
    { press: place("System"), say: "System" },
    { press: systemPage("All"), say: "All, first in Mods" },
    { look: role("group", "Dusk"), say: "Every mod on this machine, one row each, its kind by its glyph. Those switched on come first" },

    // Words and Fleet, the two kinds the first teachings made.
    { press: place("System"), say: "System" },
    { press: systemPage("Words"), say: "Mods are split by what they change: Theme, Layout, Words and Fleet" },
    { press: button("Details of jira-keys"), say: "A Words mod, opened" },
    { look: inside(role("group", "Details of jira-keys"), text("Found in pull request #2712")), say: "The pattern, and an address for each repository with where it was found" },
    { press: systemPage("Fleet"), say: "Fleet" },
    { press: button("Details of slot-behind-batched"), say: "A Fleet mod, opened" },
    { look: role("group", "Details of slot-behind-batched"), say: "The notice, what Fleet does with it, and where" },

    // Teach on a row: a theme tweak.
    toSessions,
    { context: deckRow(PAYMENTS_TITLE), say: "A right press on a row that needs him" },
    { press: menuItem("Teach"), say: "Teach" },
    { look: section("Row"), say: "What it was pressed on" },
    { type: "make these louder", into: instruction, say: "In his own words" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Theme tweak"), say: "One shared colour, and the colour on screen already" },
    { look: inside(section("Theme tweak"), role("list", "Also drawn in")), say: "Every other place that colour is drawn" },
    { press: inside(panel, button("Save")), say: "Save" },

    // The tweak sits over the theme.
    { press: place("System"), say: "System" },
    { press: systemPage("Theme"), say: "Theme" },
    { look: tweak, say: "The tweak, in its own list under Theme" },
    { type: "nord", into: theme, say: "Another theme" },
    { press: role("option", "Nord", { exact: true }), say: "Nord" },
    { look: tweak, say: "The tweak is still on" },
    { press: tweak, say: "Switched off, and the colour is the theme's again" },

    // Teach on a panel, from the keyboard: a layout change.
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "A Job" },
    { key: "Shift+F10", on: inside(field, button("Expand")), say: "Shift+F10 opens a panel's menu from the keyboard" },
    { press: menuItem("Teach"), say: "Teach" },
    { look: section("Panel Field"), say: "What it was pressed on" },
    { type: "hide this, I never look at it", into: instruction, say: "In his own words" },
    { key: "Meta+Enter", on: instruction, say: "⌘↵ teaches" },
    { look: section("Layout"), say: "A layout change, and no note for agents" },
    { press: inside(panel, button("Save")), say: "Save" },
    { look: region("Progress", { exact: true }), say: "The Field is gone" },
    { press: place("System"), say: "System" },
    { press: systemPage("Layout"), say: "Layout" },
    { look: role("group", "hide-field"), say: "The layout mod, and what it was taught on" },
  ];
}

const teachMods = walk("teach-mods", steps());

export { teachMods as "teach-mods" };
