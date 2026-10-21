// Kit's Output styles tile: each style with what it says and where it came from, the active one
// marked, and Armada's session rules set apart as always applied. Over `output-styles`.

import { inside, region, role, text, toSystemPage, walk } from "../walk";

const STYLES = region("Output styles");

const outputStyles = walk("output-styles", [
  ...toSystemPage("Output styles", "Output styles"),
  { look: STYLES, say: "Every style, with the active one marked" },
  { look: inside(STYLES, text("bluf")), say: "Your own style, in force" },
  { hover: inside(STYLES, role("img", "Active", { exact: true })), say: "The active style is marked" },
  { look: inside(STYLES, text("Yours")), say: "Where it came from, in words" },
  { look: inside(STYLES, text("armada")), say: "A plugin's style, under the plugin's name" },
  { look: inside(STYLES, text("This repository")), say: "A repository's own" },
  { look: inside(STYLES, text("Built in")), say: "One the agent carries" },
  { look: inside(STYLES, region("Always applied")), say: "Armada's job rules sit on top of the active style in jobs Bridge hosts. Not a style you pick" },
]);

export { outputStyles as "output-styles" };
