// The Cockpit powering on while Fleet restarts, over `fleet-restart`. The deck stays drawn through the restart with no banner: each
// panel stands by, then takes its data in turn as Fleet answers, and the map brings its territories in one at a time.

import { region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const map = role("listbox", "Map");

const powerOn = walk("fleet-restart", [
  { look: jobs, say: "The Cockpit, live. It powered on when the window opened: each panel drew its frame, lit its label and glyph, then took its data" },
  { later: jobs, say: "Fleet restarts. Every panel goes to standby: its body dark, its frame and label lit, its glyph dim in the middle. No banner" },
  { look: region("Bays"), say: "Fleet goes from unreachable to not running to starting. The deck holds standby through all of it, a scan line passing down each panel" },
  { look: jobs, say: "Fleet answers. Each panel's data wipes in from the top in turn, Jobs first and Autopilot last" },
  { look: map, say: "Once the map's data has settled, its territories come in one at a time, each growing from its middle. The camera stays put" },
  { later: map, say: "A bay in a third repository. Its territory grows in and the others glide to make room" },
]);

export { powerOn as "cockpit-power-on" };
