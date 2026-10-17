// The depth treatment in three colourways, `?take=teal|ember|graphite`: the Cockpit, a Session and Settings.
// Over `cockpit-ship-flightdeck`. A design mock: nothing here asserts a colour.

import { place, region, systemPage, text, walk } from "../walk";

const depthTakes = walk("cockpit-ship-flightdeck", [
  { look: region("Jobs and Sessions"), say: "Rows are tiles on glass, marks glow" },
  { press: place("Sessions"), say: "Sessions, tabs in a well, Active lit" },
  { press: text("Pin the store clock"), say: "A Session" },
  { press: place("System"), say: "System" },
  { press: systemPage("Settings"), say: "Settings" },
  { look: text("Harness"), say: "Tiles for fields, wells for inputs, raised buttons" },
]);

export { depthTakes as "depth-takes" };
