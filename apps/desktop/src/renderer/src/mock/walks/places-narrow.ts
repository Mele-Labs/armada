// The places on a narrow window: one line, each tab drawn as its glyph with its name on hover, where the row has no
// room for the labels. Over `title-bar-menu`, whose plan meters widen the row's end.

import { place, walk } from "../walk";

const placesNarrow = walk(
  "title-bar-menu",
  [
    { hover: place("Studios"), say: "Places on a narrow window: one line of glyphs, each named on hover" },
    { press: place("Studios"), say: "Studios" },
    { hover: place("System"), say: "System" },
    { press: place("System"), say: "System, with its pages under the row" },
    { press: place("Cockpit"), say: "Widen the window and the names come back" },
  ],
  { width: 980, height: 800 },
);

export { placesNarrow as "places-narrow" };
