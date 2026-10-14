// Cockpit direction B, the mission wall, over the shared ship `cockpit-ship`. Six stations, one enlarged
// for the station in trouble and the rest condensed to a mark on the edge. A Session's reply arriving as a
// transmission and answered from its card, a Peek, a reply in place, a pull request landing, main going red
// and Fix, and the stuck Session found and stopped. A throwaway prototype on proto/cockpit-three-directions.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const glass = region("Dashboard");
const station = (name: string) => region(name);
const subject = (where: string, title: string) => inside(station(where), role("listitem", title));
const tile = (name: RegExp) => button(name);
const call = (title: RegExp) => region(title);

const cockpitDirectionWall = walk("cockpit-ship", [
  { look: glass, say: "The wall at rest. Gray, and lit only where he is owed something" },
  { look: station("Sessions"), say: "Sessions takes the large tile: a stuck Session, a red pull request, a question" },
  { look: button("Main", { exact: true }), say: "Main is well: a calm tile draws nothing else" },
  { look: button("Next: Jobs"), say: "The flight director names the next station that needs him" },
  { later: glass, say: "A Session that was working replies" },
  { look: button(/Pin the store clock/), say: "It arrives as a transmission in the tray" },
  { key: "w", on: glass, say: "w opens it" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in words" },
  { look: inside(subject("Sessions", "Pin the store clock"), role("img", "Working")), say: "The Session is at work again" },
  { key: "]", on: glass, say: "] jumps to the next station in trouble" },
  { look: subject("Jobs", "Add the pause marker column"), say: "Jobs on the large tile: a review, a failed Check" },
  { press: tile(/^Sessions/), say: "And back to Sessions" },
  { key: "p", on: subject("Sessions", "Migration notes"), say: "Peek on a working Session" },
  { look: dialog("Peek at Migration notes"), say: "The end of its thread, with its working mark and its own box" },
  { look: inside(dialog("Peek at Migration notes"), text(/0044 drops the legacy index/)), say: "A row lands while it is open, and the thread follows its end" },
  { key: "Escape", on: dialog("Peek at Migration notes"), say: "Put away" },
  { key: "r", on: subject("Sessions", "Fix the reconnect wait"), say: "Reply in place on its line" },
  { type: "Fix the desktop_test failure.\n", into: role("textbox", "Reply to Fix the reconnect wait"), say: "Sent where its page's box sends" },
  { look: subject("Sessions", "Fix the reconnect wait"), say: "It is at work on the red pull request" },
  { press: tile(/^Merge line/), say: "The merge line, on the large tile again" },
  { key: "m", on: subject("Merge line", "Gate policy on every run"), say: "Merge, from its line" },
  { look: subject("Merge line", "Gate policy on every run"), say: "Landed" },
  { later: glass, say: "Main goes red" },
  { look: station("Main"), say: "Main takes the large tile on its own" },
  { press: button("Fix main in armada"), say: "Fix" },
  { look: inside(glass, role("img", "Fixing main")), say: "Handed to a Job" },
  { later: glass, say: "Its fix lands" },
  { look: button("Main", { exact: true }), say: "Main is green again, back on the edge" },
  { look: subject("Sessions", "Service worker audit"), say: "The stuck Session, found" },
  { key: "s", on: subject("Sessions", "Service worker audit"), say: "Stop" },
  { look: station("Jobs"), say: "Stopped. The next alarm takes the large tile" },
]);

export { cockpitDirectionWall };
