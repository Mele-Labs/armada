// The dispatch bar growing into the dispatch panel where it stands, and the same panel floating over
// another screen. The repository is a chip in the panel, read from the words and asked inline when
// none is found. Over `dispatch-grows-from-the-bar`.

import { button, dialog, inside, role, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const repository = (name: string) => inside(frame, button(name));
const choices = inside(frame, role("listbox", "Repository"));

const dispatchGrowsFromTheBar = walk("dispatch-grows-from-the-bar", [
  { look: request, say: "The dispatch bar over the Cockpit, the cursor in it, no repository chosen" },
  { type: "Show the Drone's branch on each Bridge cockpit tile", into: request, say: "The first words, naming Bridge" },
  { look: repository("Repository, finding"), say: "The bar has grown into the panel in place, the words and the cursor kept. The chip's ring turns while the words are read" },
  { look: repository("Repository armada"), say: "Read as the armada repository" },
  { hover: repository("Repository armada"), say: "The tooltip says where it came from" },
  { look: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch is ready" },
  { type: "", into: request, say: "The request cleared; the panel stays open" },
  { type: "Clean up the old retry logic", into: request, say: "A request that names no repository" },
  { look: repository("Repository, not found"), say: "The chip stays an empty dashed outline" },
  { hover: repository("Repository, not found"), say: "The tooltip names it" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch builds nothing without a repository" },
  { look: choices, say: "The panel asks in place: the frame and band take the call's hue and the set-up repositories list, armada first" },
  { press: inside(choices, role("option", /storefront/)), say: "Picking one sends the request" },
  { look: request, say: "The panel is the bar again, the request empty" },
  { press: button("Studios", { exact: true }), say: "Another screen" },
  { key: "Meta+n", on: button("Studios", { exact: true }), say: "⌘N" },
  { look: inside(dialog("Dispatch"), request), say: "The same panel floats over it, top-anchored, and grows from its line" },
  { type: "Pin the retry clock in the store tests", into: inside(dialog("Dispatch"), request), say: "A request typed here" },
  { look: inside(dialog("Dispatch"), repository("Repository, finding")), say: "The chip reads the words again" },
  { key: "Escape", on: inside(dialog("Dispatch"), request), say: "Esc puts the panel away and keeps the request" },
  { key: "Meta+n", on: button("Studios", { exact: true }), say: "⌘N again" },
  { look: inside(dialog("Dispatch"), request), say: "The panel opens holding the request, the cursor at its end" },
]);

export { dispatchGrowsFromTheBar as "dispatch-grows-from-the-bar" };
