// One timestamp format, set once in Settings and used wherever Bridge draws a time. A press on a
// stamp turns every stamp on that screen between the clock and the age. Over `session-time-format`.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, role, toSystemPage, walk } from "../walk";

const thread = region("Thread");
const clockStamp = inside(thread, button(/\d:\d\d/));
const ageStamp = inside(thread, button(/ago$/));
const session = inside(region("Jobs", { exact: true }), button(/Retry backoff/));
const dated = button(/^[A-Z][a-z]{2} \d{1,2} \d/);

export const timeFormat = walk("session-time-format", [
  toSessions,
  { press: session, say: "A Session with a thread" },
  { look: clockStamp, say: "Each row carries its time" },
  { press: clockStamp, say: "Pressed, every time in the thread reads as an age" },
  { look: ageStamp, say: "Ages" },
  { press: ageStamp, say: "Pressed again, the clocks are back" },
  ...toSystemPage("Settings", "Settings, from the System strip"),
  { press: role("tab", "Appearance"), say: "Appearance, the group Time is in" },
  { press: role("switch", "24-hour clock"), say: "24-hour clock, off" },
  { press: role("switch", "Seconds"), say: "Seconds, off" },
  { press: role("switch", "Date"), say: "Date, on" },
  { look: dated, say: "The sample is now in that format" },
  { press: place("Cockpit"), say: "Cockpit" },
  { press: session, say: "The same Session" },
  { look: inside(thread, dated), say: "Every stamp is in the new format" },
  { look: inside(thread, dated), say: "Tab reaches a stamp, Enter or Space turns the thread to ages" },
]);
