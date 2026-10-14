// Cockpit direction B as a navigation model, over the shared ship `cockpit-ship`. The Cockpit holds the places the
// Work group was; a press or `g` and a key takes the real page as a layer, and Back returns to the Cockpit
// as it was left. A throwaway prototype on proto/cockpit-three-directions.

import { button, inside, region, role, text, walk } from "../walk";

const cockpit = region("Cockpit");
const station = (name: string) => inside(cockpit, region(name));
const strip = (where: string, title: string) => inside(station(where), role("listitem", title));
const bar = region("Layer bar");

const cockpitDirectionApp = walk("cockpit-ship", [
  { look: cockpit, say: "The Cockpit at rest. The left rail keeps Machine and Fleet; the Work group is in here" },
  { key: "p", on: strip("Sessions", "Migration notes"), say: "Peek on a working Session" },
  { look: region("Peek at Migration notes"), say: "Left open" },
  { key: "g", on: cockpit, say: "g, then a place: with the Peek still open" },
  { key: "s", on: cockpit, say: "s is Sessions" },
  { look: bar, say: "The real Sessions page, under a layer bar" },
  { press: button(/Fix the reconnect wait/), say: "Open a Session from the page" },
  { look: bar, say: "The real thread takes the area; the bar keeps the way back" },
  { type: "Fix the desktop_test failure.\n", into: role("textbox"), say: "Answered in the thread" },
  { look: text(/Fix the desktop_test failure/), say: "Sent where the page sends" },
  { later: bar, say: "A Session replies while this one is open" },
  { look: button(/Pin the store clock/), say: "Its call arrives in the tray, over the layer" },
  { key: "w", on: bar, say: "w opens it over the layer" },
  { type: "Good. Merge it.\n", into: inside(region(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered" },
  { look: inside(bar, text("Fix the reconnect wait")), say: "Back on the Session it was over" },
  { press: button("Back to Sessions"), say: "Back pops one layer" },
  { press: button("Back to Cockpit"), say: "And the next" },
  { look: region("Peek at Migration notes"), say: "The Cockpit as it was left: the Peek is still open" },
  { key: "Escape", on: region("Peek at Migration notes"), say: "Put away" },
  { key: "g", on: cockpit, say: "g, then a place" },
  { key: "m", on: cockpit, say: "m is the merge line" },
  { look: bar, say: "The real merge line page, and a ready pull request in the bar" },
  { key: "?", on: bar, say: "? lists every key" },
  { look: region("Keys"), say: "A digit for each place, [ and ] to step, . and , for trouble" },
  { key: "?", on: bar, say: "Put away" },
  { press: button("Merge #1890"), say: "Merge it from the bar" },
  { key: "2", on: bar, say: "2 is Sessions, one key: it replaces the merge line layer" },
  { look: inside(bar, text("Sessions")), say: "Sessions where the merge line was, one layer deep" },
  { press: button("Back to Cockpit"), say: "Back" },
  { look: station("Sessions"), say: "The Cockpit, where it was" },
  { key: "g", on: cockpit, say: "g, then a place" },
  { key: "j", on: cockpit, say: "j is Jobs" },
  { later: bar, say: "Main goes red while the Jobs page is open" },
  { look: inside(bar, button(/^Main: desktop_test failing/)), say: "Its mark on the bar goes red" },
  { press: inside(bar, button(/^Main: desktop_test failing/)), say: "Press it" },
  { look: station("Main"), say: "The Cockpit, with main on the large tile" },
  { press: button("Fix main in armada"), say: "Fix" },
  { look: inside(cockpit, role("img", "Fixing main")), say: "Handed to a Job" },
  { later: cockpit, say: "Its fix lands" },
  { key: ".", on: cockpit, say: ". goes to the next station in trouble" },
  { look: station("Sessions"), say: "The worst first: a stuck Session" },
  { key: ".", on: cockpit, say: ". again" },
  { press: inside(strip("Jobs", "Add the pause marker column"), button("Open")), say: "Open a Job" },
  { look: inside(bar, text("Add the pause marker column")), say: "The real Job detail takes the area" },
  { key: "Escape", on: bar, say: "Esc pops the layer" },
  { look: station("Jobs"), say: "Back on the Cockpit, on Jobs" },
]);

export { cockpitDirectionApp };
