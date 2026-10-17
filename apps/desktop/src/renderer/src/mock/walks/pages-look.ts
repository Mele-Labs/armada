// Retros, Studios and Doctor in the new look, `?look=new`: a page title, panels with a tracked caps head and hairlines,
// the Cockpit's frame. A design mock: nothing here asserts a colour or a size.

import { button, toSystemPage, viaPalette, walk } from "../walk";

const pagesLook = walk("pages-look", [
  ...toSystemPage("Retros", "Retros"),
  { press: button("Evidence", { exact: true }), say: "A lesson, and its evidence in a well" },
  ...toSystemPage("Studios", "Studios"),
  { press: button("Open", { exact: true }), say: "The board in its panel" },
  ...viaPalette("Doctor", "Doctor"),
]);

export { pagesLook as "pages-look" };
