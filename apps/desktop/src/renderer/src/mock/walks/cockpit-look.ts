// The Cockpit's panels and rows in Bridge's look: one frame and header, calls, rows, the repository combobox.
// Over `cockpit-list-by-repository`. A design mock: nothing here asserts a colour or a size.

import { inside, region, role, text, walk } from "../walk";

const list = region("Work", { exact: true });
const calls = region("Incoming calls");

const cockpitLook = walk("cockpit-list-by-repository", [
  { look: list, say: "Every panel has one frame and one header" },
  { hover: inside(list, text("The retry loop")), say: "A row under the pointer rises" },
  { hover: inside(calls, text("Prune the stale branches")), say: "A call reads kind, subject, age" },
  { press: inside(calls, text("Prune the stale branches")), say: "The opened Session is lit in the list" },
  { press: role("combobox", "Repository"), say: "The field becomes a search box and the list opens" },
  { type: "store", into: role("combobox", "Repository"), say: "The list narrows as it is typed" },
]);

export { cockpitLook as "cockpit-look" };
