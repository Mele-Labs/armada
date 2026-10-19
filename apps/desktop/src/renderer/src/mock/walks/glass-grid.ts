// The Cockpit's panels as one sheet of glass over a grid, with a Job open: its Field and Progress on the right, Flight control across the foot.
// Over `one-session-view`. Takes with `?grid=lines|dots|blueprint`. A design mock: nothing here asserts a colour or a size.

import { deckRow, inside, region, role, walk } from "../walk";

const stage = region(/^Job: /);

const glassGrid = walk("one-session-view", [
  { look: region("Jobs", { exact: true }), say: "Every panel is one sheet: the same fill, edge, corner, shadow and header, over one grid" },
  { press: deckRow("Retry the slot lease"), say: "A Job opens in the centre, with the Field and Progress beside it" },
  { look: inside(stage, role("textbox", "Message")), say: "The thread is the sheet itself. The message box is a flat field on it" },
  { key: "f", on: region("Jobs", { exact: true }), say: "F moves focus to Flight control. Its edge and shadow lift, and no header fills" },
  { press: role("combobox", "Repository"), say: "The repository field opens its list" },
  { look: role("listbox", "Repositories"), say: "The list floats over the panels as the same glass at the overlay level" },
]);

export { glassGrid as "glass-grid" };
