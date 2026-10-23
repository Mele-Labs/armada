// A Job whose flight controls outgrow two rows: seventeen tiles in pages of six columns by two rows that snap sideways, a dot for each page, and
// Edit paging the same way with `+` last. Over `flight-controls-overflow`. The owner's ask of 23 Oct 2026.

import { button, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });
const controls = region("Flight controls");
const editing = inside(controls, role("listbox", "Flight controls, editing"));

const flightControlsOverflow = walk("flight-controls-overflow", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with seventeen flight controls" },
  { look: inside(controls, button("Page 2")), say: "Twelve tiles to a page, and a dot for each page" },
  { press: inside(controls, button("Page 2")), say: "A press on a dot, or a swipe, brings the page in" },
  { press: inside(controls, button("Edit")), say: "Edit pages the same way" },
  { look: inside(editing, role("option", "Add action")), say: "The + tile is the last one, on the last page" },
]);

export { flightControlsOverflow as "flight-controls-overflow" };
