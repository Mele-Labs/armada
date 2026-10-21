// A right press on anything opens a menu with Report, and Report opens a note on what was pressed,
// with the annotation layer off. A Job's row has a menu of its own, and Report is in it. Two walks
// over the `report-from-right-click` scenario, each from the layer off: a note on, the layer stays on.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const writes = (what: string): Step[] => [
  { type: what, into: inside(dialog("New note"), role("textbox", "Note")), say: "What is wrong with it" },
  { press: button("Save note"), say: "Saved" },
  { look: inside(dialog("Note"), text(what)), say: "The note stays up where it was left" },
];

const anything: Step[] = [
  { context: region("Jobs", { exact: true }), say: "A right press on anything the app has no menu for" },
  { look: role("menuitem", "Report"), say: "Report is in the menu" },
  { press: role("menuitem", "Report"), say: "Report opens a note on what was pressed" },
  ...writes("The list needs a heading"),
];

const row: Step[] = [
  { context: deckRow("Fix the flaky store test"), say: "A Job's row has a menu of its own" },
  { look: role("menuitem", "Auto Pilot"), say: "With its own acts" },
  { look: role("menuitem", "Report"), say: "And Report, always" },
  { press: role("menuitem", "Report"), say: "Report opens a note on the row" },
  ...writes("The row should say which step it is on"),
];

const planned = walk("report-from-right-click", anything);
const onARow = walk("report-from-right-click", row);
export { planned as "report-from-right-click", onARow as "report-from-job-row" };
