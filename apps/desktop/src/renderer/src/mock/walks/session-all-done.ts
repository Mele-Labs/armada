// A Session whose agent says it has nothing left: the thread ends in a callout. Keep going hides it and
// puts the cursor in the message box; End session ends the Session. Over the `session-all-done` scenario.
// Told twice each: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

const callout = role("article", "Nothing left to do");

function opened(): Step[] {
  const mine = inside(region("Sessions"), button(/Pin the store clock/));
  return [
    toSessions,
    { press: mine, say: "Opened" },
    { look: callout, say: "The callout, last in the thread" },
    { hover: inside(callout, role("img", "Nothing left to work on")), say: "The mark, and on hover what it says" },
  ];
}

const keep: Step[] = [
  ...opened(),
  { press: inside(callout, button("Keep going")), say: "Keep going hides it" },
  { look: kit(false).message, say: "The cursor is in the message box" },
];

const end: Step[] = [
  ...opened(),
  { press: inside(callout, button("End session")), say: "End session ends the Session" },
  { look: role("img", "Ended"), say: "Ended" },
];

const wide = walk("session-all-done", keep);
const narrow = walk("session-all-done", keep, NARROW);
const wideEnd = walk("session-all-done", end);
const narrowEnd = walk("session-all-done", end, NARROW);

export {
  wide as "session-all-done",
  narrow as "session-all-done-narrow",
  wideEnd as "session-all-done-end",
  narrowEnd as "session-all-done-end-narrow",
};
