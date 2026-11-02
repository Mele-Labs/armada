// A mod's instrument on the console: one that faulted, a new one made from the tray, and its Job, which lives on the bay and takes a row only while no bay can show its question. Over `console-instrument-mods`.

import { button, inside, region, role, walk } from "../walk";

const controls = region("Flight controls");
const list = region("Jobs", { exact: true });
const tray = role("listbox", "Instruments");
const option = (name: string) => inside(tray, role("option", name, { exact: true }));
const sheet = role("dialog", "New instrument");
const built = inside(controls, button("Main build"));

const consoleInstrumentMods = walk("console-instrument-mods", [
  { look: inside(controls, button("Reviews")), say: "Reviews is a mod in bay 1, seated from the rack and lit like Armada's own" },
  { hover: inside(controls, button("Spend")), say: "Spend threw as it drew, so bay 3 is an empty socket that says why. Its seat is kept for when it works again" },
  { press: inside(controls, role("button", "Edit", { exact: true })), say: "Edit opens the rack" },
  { press: role("button", /^Bay 4/), say: "Pick bay 4" },
  { press: option("Make one"), say: "Make one, at the end of the tray" },
  { type: "How long ci on main has taken, last 16 runs", into: inside(sheet, role("textbox", "New instrument")), say: "Say what it shows" },
  { press: inside(sheet, button("Send")), say: "This starts its Job" },
  { press: inside(controls, role("button", "Done", { exact: true })), say: "Done" },
  { look: built, say: "The bay shows it being built, and its Job has no row in the Jobs list" },
  { hover: built, say: "Its Job's card opens from the bay" },
  { look: role("button", "Allow once"), say: "The Job has a question, so the bay lights" },
  { press: inside(list, button("Fix the reconnect wait")), say: "Open another Job. The console gives way to that Job's keys, so no bay can show the question" },
  { look: inside(list, button("Mod Main build")), say: "So the Job takes a pulsing row, only for as long as that lasts" },
  { press: inside(region(/^Job /), button("Back", { exact: true })), say: "Close the other Job" },
  { hover: built, say: "The console is back, the row has gone and the bay lights again" },
  { press: role("button", "Allow once"), say: "Answer it on the card" },
  { look: built, say: "It finishes and the face takes the bay" },
  { press: built, say: "A press opens its panel" },
  { look: role("dialog", "Main build"), say: "Over the field, where Armada's own instruments open theirs" },
]);

export { consoleInstrumentMods as "console-instrument-mods" };
