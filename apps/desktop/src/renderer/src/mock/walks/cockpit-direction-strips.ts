// Cockpit direction A, the strips board, over the shared ship `cockpit-ship`. Strips in columns by what they
// stand on, Needs you, Working, Stuck or held, Done; the merge line as the one pipeline, a rail of pull
// requests along the foot ending at main. A Session's reply arriving as a transmission and answered from its
// card, a Peek, a reply in place on a strip, a pull request merged from its unit, main going red and Fix, and
// the stuck Session found and stopped. A throwaway prototype on proto/cockpit-three-directions.

import { button, inside, region, role, text, walk } from "../walk";

const glass = region("Dashboard");
const column = (name: string) => inside(glass, role("list", name));
const strip = (list: string, title: string) => inside(column(list), role("listitem", title));
const rail = inside(glass, region("Merge line"));
const unit = (stage: string, title: string) => inside(rail, inside(role("list", stage), role("listitem", title)));
const call = (title: RegExp) => region(title);

const cockpitDirectionStrips = walk("cockpit-ship", [
  { look: glass, say: "The ship at rest. Gray, and lit only where he is owed something" },
  { look: column("Needs you"), say: "Needs you: a question, a done, a review, a failed Check, a red pull request" },
  { look: inside(glass, role("img", "Main", { exact: true })), say: "Roll call: main, the merge line, the bays and Autopilot as marks" },
  { look: strip("Needs you", "Add the pause marker column"), say: "A Job awaiting review: its workflow is the step bar inside the strip" },
  { look: unit("Checking", "Add the pause marker column"), say: "The merge line is only pull requests, left to right to main" },
  { look: strip("Stuck or held", "Service worker audit"), say: "A Session that has gone quiet is held, whatever it was doing" },
  { later: glass, say: "A Session that was working replies" },
  { look: button(/Pin the store clock/), say: "It arrives as a transmission in the tray" },
  { key: "w", on: glass, say: "w opens it" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in words" },
  { look: strip("Working", "Pin the store clock"), say: "The Session is at work again, in Working" },
  { key: "p", on: strip("Working", "Migration notes"), say: "Peek on a working Session" },
  { look: region("Peek at Migration notes"), say: "The end of its thread, with its working mark and its own box" },
  { look: inside(region("Peek at Migration notes"), text(/0044 drops the legacy index/)), say: "A row lands while it is open, and the thread follows its end" },
  { key: "Escape", on: region("Peek at Migration notes"), say: "Put away" },
  { key: "r", on: strip("Needs you", "Fix the reconnect wait"), say: "Reply in place on its strip" },
  { type: "Fix the desktop_test failure.\n", into: role("textbox", "Reply to Fix the reconnect wait"), say: "Sent where its page's box sends" },
  { look: strip("Working", "Fix the reconnect wait"), say: "It is at work on the red pull request" },
  { hover: strip("Working", "Gate policy on every run"), say: "Hover a strip and its pull request lights on the rail" },
  { look: unit("In queue", "Gate policy on every run"), say: "Joined to its owner by the chip and by the light" },
  { press: button("Merge #1890"), say: "Merge, from the pull request's unit" },
  { look: unit("Landed", "Gate policy on every run"), say: "Landed, at main" },
  { later: glass, say: "Main goes red" },
  { look: inside(rail, role("img", /^Main: desktop_test failing/)), say: "Red lands on the main marker on its own: the alarm" },
  { press: button("Fix main in armada"), say: "Fix" },
  { look: inside(glass, role("img", "Fixing main")), say: "Handed to a Job" },
  { later: glass, say: "Its fix lands" },
  { look: inside(rail, role("img", "Main", { exact: true })), say: "Main is green again" },
  { look: strip("Stuck or held", "Service worker audit"), say: "The stuck Session, found" },
  { key: "s", on: strip("Stuck or held", "Service worker audit"), say: "Stop" },
  { look: strip("Done", "Service worker audit"), say: "Stopped, and gray" },
]);

export { cockpitDirectionStrips };
