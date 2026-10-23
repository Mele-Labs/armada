// A flight control's key, set in its sheet by pressing it: a digit, a letter or a chord such as ⌥1. A key the Cockpit already answers is named and
// holds Save; a saved one presses its tile from anywhere on the Cockpit while a Job is open. The owner's ask of 23 Oct 2026.

import { button, dialog, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const tile = (name: string) => inside(controls, button(name));
const editing = inside(controls, role("listbox", "Flight controls, editing"));
const sheet = dialog("Change Report issue with job");
const keyField = inside(sheet, button(/^Key:/));

const flightControlsHotkeys = walk("flight-controls-custom", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job open on the Cockpit" },
  { press: tile("Edit"), say: "Edit turns the tiles into things to arrange" },
  { press: inside(editing, role("option", "Report issue with job")), say: "Any tile opens its sheet, Pause too" },
  { press: keyField, say: "The Key is a control: press it, then press the key" },
  { key: "Alt+r", on: keyField, say: "A digit, a letter or a chord with ⌘, ⌃ or ⌥. Esc lets go, Delete clears" },
  { press: keyField, say: "Another key" },
  { key: "m", on: keyField, say: "A key the Cockpit already answers is named under the control, and Save waits" },
  { press: keyField, say: "Again" },
  { key: "Alt+r", on: keyField, say: "Back to the first" },
  { press: inside(sheet, button("Save")), say: "Saved" },
  { look: inside(editing, role("option", "Report issue with job")), say: "The tile wears the chord in its corner" },
  { press: tile("Done"), say: "Done" },
  { key: "Alt+r", on: list, say: "The chord presses the tile from anywhere on the Cockpit while the Job is open" },
  { look: dialog("Report an issue with the Job"), say: "Report opens its box" },
]);

export { flightControlsHotkeys as "flight-controls-hotkeys" };
