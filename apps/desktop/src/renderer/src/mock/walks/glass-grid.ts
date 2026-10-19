// The Cockpit's panels as one sheet of glass over a grid, with a Job open: its Field and Progress on the right, Flight control across the foot.
// Over `one-session-view`. Takes with `?grid=lines|dots|blueprint` and `?glass=current|vision|vision-tight`. A design mock: nothing here asserts a colour or a size.

import { button, inside, region, role, walk } from "../walk";

const stage = region(/^Job: /);

const glassGrid = walk("one-session-view", [
  { look: role("listbox", "Map"), say: "With no Job open the map has no panel round it. The grid shows round its floor, and the panels sit around it" },
  { look: region("Jobs", { exact: true }), say: "Every other panel is one sheet: the same fill, edge, corner, shadow and header, over the same grid" },
  { press: inside(region("Jobs", { exact: true }), button("Retry the slot lease", { exact: true })), say: "A Job opens in the centre, with the Field and Progress beside it" },
  { look: inside(stage, role("textbox", "Message")), say: "The thread is the sheet itself. The message box is a flat field on it" },
  { press: region("Flight controls"), say: "Focus moves to Flight control" },
  { look: region("Flight controls"), say: "The focused panel lifts its edge and shadow. No header fills" },
  { press: role("combobox", "Repository"), say: "The repository field opens its list" },
  { look: role("listbox", "Repositories"), say: "The list floats over the panels as glass at the overlay level" },
]);

export { glassGrid as "glass-grid" };
