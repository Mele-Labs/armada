// The restyle in one pass: the Cockpit at rest, the repository picker, the inspect card
// on a row, a bay, a call and a drone, the Session's context tank pinned, the command palette, then Doctor, Settings, Worktrees, Retros and Studios.
// Over `restyle-review`. A design mock: nothing here asserts a colour or a size.

import { button, inside, region, role, toSystemPage, viaPalette, walk } from "../walk";

const list = region("Jobs", { exact: true });
const bays = region("Bays");
const picker = role("button", "Repository", { exact: true });
const find = role("searchbox", "Find a repository");
const context = button(/^Context 61%/);

const restyleReview = walk("restyle-review", [
  { look: list, say: "Panels share one frame and one header, with the title bar on the canvas" },
  { look: inside(list, button("Cache key")), say: "A Job that needs him is a yellow row that pulses" },
  { press: picker, say: "The title row's picker opens a search box. All leads, a rule, then the repositories" },
  { type: "arm", into: find, say: "The list narrows as it is typed" },
  { key: "ArrowUp", on: find, say: "Add a repository is the last row, outside the filter" },
  { key: "Escape", on: find, say: "Esc closes the list" },
  { hover: inside(list, button("Changelog script")), say: "A row opens the inspect card" },
  { hover: inside(bays, button(/^slot-3, armada, Held/)), say: "A bay opens the same card" },
  { hover: inside(list, button("Cache key")), say: "So does a row that is calling him" },
  { hover: role("option", /^Pin the store clock, Job/), say: "And a drone on the map" },
  { press: inside(list, button("Migration notes")), say: "A Job opens in the area" },
  { press: context, say: "The context tank pins its card" },
  { key: "Escape", on: role("dialog", "Context"), say: "Esc closes it" },
  { key: "Meta+k", on: role("navigation", "Places"), say: "⌘K opens the palette" },
  { look: role("dialog", "Command palette"), say: "Input row over a hairline, small-cap sections, compact rows, keycaps at the right" },
  { key: "Escape", on: inside(role("dialog", "Command palette"), role("combobox")), say: "Esc closes it" },
  ...viaPalette("Doctor", "Doctor"),
  ...toSystemPage("Settings", "Settings"),
  ...viaPalette("Worktrees", "Worktree slots"),
  ...toSystemPage("Retros", "Retros"),
  { press: button("Evidence", { exact: true }), say: "A lesson and its evidence" },
  ...toSystemPage("Studios", "Studios"),
]);

export { restyleReview as "restyle-review" };
