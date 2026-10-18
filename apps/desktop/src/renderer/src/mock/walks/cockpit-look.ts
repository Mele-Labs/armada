// The Cockpit's panels and rows in Bridge's look: one frame and header, calls, rows, the repository combobox.
// Over `cockpit-list-by-repository`. A design mock: nothing here asserts a colour or a size.

import { inside, region, role, text, walk } from "../walk";

const list = region("Work", { exact: true });

const cockpitLook = walk("cockpit-list-by-repository", [
  { look: list, say: "Every panel has one frame and one header" },
  { hover: inside(list, text("Draft the migration notes")), say: "A row under the pointer rises" },
  { hover: inside(list, text("Prune the stale branches")), say: "A row that is calling him pulses and its card names the call" },
  { press: inside(list, text("Prune the stale branches")), say: "The opened Session is lit in the list" },
  { press: role("combobox", "Repository"), say: "The field becomes a search box and the list opens" },
  { type: "store", into: role("combobox", "Repository"), say: "The list narrows as it is typed" },
]);

export { cockpitLook as "cockpit-look" };
