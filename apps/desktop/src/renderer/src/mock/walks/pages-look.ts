// Retros, Studios and Doctor in Bridge's look: a page title, panels with a tracked caps head and hairlines,
// the Cockpit's frame. A design mock: nothing here asserts a colour or a size.

import { button, systemPage, toSystemPage, viaPalette, walk } from "../walk";

const pagesLook = walk("pages-look", [
  ...toSystemPage("Retros", "Retros"),
  { press: button("Evidence", { exact: true }), say: "A lesson, and its evidence in a well" },
  ...toSystemPage("Studios", "Studios"),
  { press: button("Open", { exact: true }), say: "The board in its panel" },
  ...viaPalette("Doctor", "Doctor"),
  ...toSystemPage("Kit", "Kit"),
  { press: systemPage("Mods"), say: "Mods" },
  { press: systemPage("Settings"), say: "Settings" },
  { press: systemPage("Guides"), say: "Guides" },
  { press: systemPage("Workflows"), say: "Workflows" },
  ...viaPalette("Worktrees", "Worktree slots"),
]);

export { pagesLook as "pages-look" };
