// The Cockpit powering on while Fleet restarts, over `fleet-restart`. The deck stays drawn through the restart with no banner: each
// panel stands by, then takes its data in turn as Fleet answers, and the map brings its territories in one at a time. Takes of the
// look: `&power=sweep` and `&power=step` beside the trace this draws.

import { region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const map = role("listbox", "Map");

const powerOn = walk("fleet-restart", [
  { look: jobs, say: "The Cockpit, live. It powered on when the window opened: each panel drew its frame, lit its label, then took its data" },
  { later: jobs, say: "Fleet restarts. Every panel goes to standby: its body dark, its frame and label lit, a ruler where its data goes. No banner" },
  { look: region("Bays"), say: "Fleet goes from unreachable to not running to starting. The deck holds standby through all of it" },
  { look: jobs, say: "Fleet answers. The panels take their data in turn, Jobs first and Autopilot last" },
  { look: map, say: "Once the map's data has settled, its territories come in one at a time, each growing from its middle. The camera stays put" },
  { later: map, say: "A bay in a third repository. Its territory grows in and the others glide to make room" },
]);

export { powerOn as "cockpit-power-on" };
