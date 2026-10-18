// The restyle in one pass: the Cockpit at rest, the repository combobox, the inspect card
// on a row, a bay, a call and a drone, the Session's context tank pinned, the command palette, then Doctor, Settings, Worktrees, Retros and Studios.
// Over `restyle-review`. A design mock: nothing here asserts a colour or a size.

import { button, inside, region, role, toSystemPage, viaPalette, walk } from "../walk";

const list = region("Jobs", { exact: true });
const calls = region("Incoming calls");
const bays = region("Bays");
const combobox = role("combobox", "Repository");
const context = button(/^Context 61%/);

const restyleReview = walk("restyle-review", [
  { look: list, say: "Panels share one frame and one header, with the title bar on the canvas" },
  { look: inside(calls, button(/^Job question: Cache key/)), say: "A call reads kind, subject and age" },
  { press: combobox, say: "The field becomes a search box. All leads, a rule, then the repositories" },
  { type: "arm", into: combobox, say: "The list narrows as it is typed" },
  { key: "ArrowUp", on: combobox, say: "Add a repository is the last row, outside the filter" },
  { key: "Escape", on: combobox, say: "Esc puts the previous value back" },
  { hover: inside(list, button("Changelog script")), say: "A row opens the inspect card" },
  { hover: inside(bays, button(/^slot-3, armada, Held/)), say: "A bay opens the same card" },
  { hover: inside(calls, button(/^Job question: Cache key/)), say: "So does a call" },
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
