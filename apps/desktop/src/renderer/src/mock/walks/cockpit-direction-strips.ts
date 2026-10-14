// Cockpit direction A, the strips board, over the shared ship `cockpit-ship`. The ship at rest, a Session's
// reply arriving as a transmission and answered from its card, a Peek, a reply in place on a strip, a
// pull request landing from its strip, main going red and Fix, and the stuck Session found and stopped.
// A throwaway prototype on proto/cockpit-three-directions.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const glass = region("Dashboard");
const column = (name: string) => inside(glass, role("list", name));
const strip = (list: string, title: string) => inside(column(list), role("listitem", title));
const call = (title: RegExp) => region(title);

const cockpitDirectionStrips = walk("cockpit-ship", [
  { look: glass, say: "The ship at rest. Gray, and lit only where he is owed something" },
  { look: column("Needs you"), say: "Needs you: a question, a done, a stuck Session, a red pull request, a review, a failed Check" },
  { look: inside(glass, role("img", "Main", { exact: true })), say: "Roll call: main, the merge line, the bays and Autopilot as marks" },
  { look: strip("Needs you", "Add the pause marker column"), say: "A Job awaiting review, its pull request a chip on it" },
  { later: glass, say: "A Session that was working replies" },
  { look: button(/Pin the store clock/), say: "It arrives as a transmission in the tray" },
  { key: "w", on: glass, say: "w opens it" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in words" },
  { look: strip("Working", "Pin the store clock"), say: "The Session is at work again, in Working" },
  { key: "p", on: strip("Working", "Migration notes"), say: "Peek on a working Session" },
  { look: dialog("Peek at Migration notes"), say: "The end of its thread, with its working mark and its own box" },
  { look: inside(dialog("Peek at Migration notes"), text(/0044 drops the legacy index/)), say: "A row lands while it is open, and the thread follows its end" },
  { key: "Escape", on: dialog("Peek at Migration notes"), say: "Put away" },
  { key: "r", on: strip("Needs you", "Fix the reconnect wait"), say: "Reply in place on its strip" },
  { type: "Fix the desktop_test failure.\n", into: role("textbox", "Reply to Fix the reconnect wait"), say: "Sent where its page's box sends" },
  { look: strip("Checking", "Fix the reconnect wait"), say: "It is at work on the red pull request: Checking" },
  { key: "m", on: strip("In queue", "Gate policy on every run"), say: "Merge, from its strip" },
  { look: strip("Landed", "Gate policy on every run"), say: "Landed" },
  { later: glass, say: "Main goes red" },
  { look: inside(column("Needs you"), role("listitem", "Main is red in armada")), say: "Red surfaces on its own, at the head of Needs you" },
  { press: button("Fix main in armada"), say: "Fix" },
  { look: inside(glass, role("img", "Fixing main")), say: "Handed to a Job" },
  { later: glass, say: "Its fix lands" },
  { look: inside(glass, role("img", "Main", { exact: true })), say: "Main is green again" },
  { look: strip("Needs you", "Service worker audit"), say: "The stuck Session, found" },
  { key: "s", on: strip("Needs you", "Service worker audit"), say: "Stop" },
  { look: strip("Working", "Service worker audit"), say: "Stopped, and gray" },
]);

export { cockpitDirectionStrips };
