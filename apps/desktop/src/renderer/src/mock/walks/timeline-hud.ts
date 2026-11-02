// The Cockpit's timeline as a HUD over the map: stuck Jobs on top, then one row for each Job that came to something since he last
// closed it, and a lamp on the TLM key while there is something new. Over `timeline-hud`.

import { button, inside, region, role, walk } from "../walk";

const controls = region("Flight controls");
const hud = role("dialog", "Timeline");
const stuck = inside(hud, role("list", "Stuck"));
const moved = inside(hud, role("list", /^Since /));

const timelineHud = walk("timeline-hud", [
  { look: inside(controls, button("Timeline")), say: "The TLM key has a lamp in its corner: something came in since he last closed the timeline" },
  { press: inside(controls, button("Timeline")), say: "TLM, or t, opens the timeline as a HUD over the map. The map keeps drawing behind it" },
  { look: stuck, say: "Stuck Jobs on top, from every repository whatever the list is narrowed to: what each asks, or the check that failed" },
  { look: moved, say: "Under them, one row for each Job that came to something since he last closed it, the newest first: merged, finished, and main red and green. A Job only working has no row" },
  { key: "ArrowDown", on: hud, say: "Up and down move along the rows, Home and End to either end" },
  { key: "End", on: hud, say: "The last row that opens anything. Main and a Job already gone have no press" },
  { key: "Enter", on: hud, say: "Enter opens that Job in the area" },
  { look: region("Job: Migration notes"), say: "The Job, in the same place as from the list" },
  { key: "Escape", on: region("Job: Migration notes"), say: "Esc comes back to the HUD, on the row he left, with the list still unread" },
  { look: inside(moved, button(/Migration notes/)), say: "Same rows, same place" },
  { key: "Escape", on: hud, say: "Esc closes it, and closing it is what marks the list read. The lamp goes out" },
  { press: inside(controls, button("Timeline")), say: "Opened again, only what is still stuck is here. The rest has been read" },
  { key: "Escape", on: hud, say: "Esc puts it away" },
]);

export { timelineHud as "timeline-hud" };
