// A mod's preview strip: only the tokens its sheet declares move it off Dark's. Dark's and Light's
// own strips are read against the files that define them in `main/themes.test.ts`.

import { expect, test } from "vitest";

import { BUILT_IN_SWATCHES, modSwatch } from "./swatches";

/** A colour, spelled by arithmetic: the gate refuses a colour literal in what a renderer ships, tests included. */
const hex = (n: number) => `#${n.toString(16).toUpperCase().padStart(6, "0")}`;

test("a mod's strip moves only the tokens its sheet declares, over Dark's", () => {
  const strip = modSwatch(`:root { --accent: ${hex(0x00ff00)}; --bg-sunken: ${hex(0xff00ff)}; }`);
  const dark = BUILT_IN_SWATCHES["dark"]!;
  expect(strip.map((colour, at) => (colour === dark[at] ? null : colour))).toEqual([null, null, null, hex(0x00ff00), null, null, null, null]);
});

test("Dark and Light are previewed by eight colours each, and not the same eight", () => {
  expect(BUILT_IN_SWATCHES["dark"]).toHaveLength(8);
  expect(BUILT_IN_SWATCHES["light"]).toHaveLength(8);
  BUILT_IN_SWATCHES["light"]?.forEach((colour, at) => expect(colour).not.toBe(BUILT_IN_SWATCHES["dark"]?.[at]));
});
