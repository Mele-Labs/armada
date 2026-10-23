// A Job whose flight controls outgrow two rows: seventeen tiles, and what shows there are more than the two rows in view. Two takes over
// `flight-controls-overflow`, chosen on `?take=` (`cockpit/deck/flight-take.ts`): `wider` grows the columns so every tile is on two
// rows, and `peek` keeps six columns with a chevron at the foot while tiles sit below. The owner's ask of 23 Oct 2026.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const editing = inside(controls, role("listbox", "Flight controls, editing"));

const flightControlsOverflow = walk("flight-controls-overflow", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with seventeen flight controls" },
  { look: controls, say: "Seventeen tiles in the panel" },
  { press: inside(controls, button("Edit")), say: "Edit lays the tiles out as they are" },
  { look: editing, say: "The same columns, with the hidden tiles and + after the rest" },
]);

export { flightControlsOverflow as "flight-controls-overflow" };
