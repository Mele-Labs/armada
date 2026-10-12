// A Session whose agent says it has nothing left: the thread ends in a callout. Under its band it lists
// the pull requests, merged first and each with its state and checks, and where there are none, the pages, files and Docs it made; each is a press.
// Keep going hides it and puts the cursor in the message box; End session ends the Session, and its
// ledger stays to be read. Over the `session-all-done` and `session-all-done-artifacts` scenarios.
// Told twice each: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

const callout = role("article", "Nothing left to do");

function opened(title: RegExp): Step[] {
  const mine = inside(region("Sessions"), button(title));
  return [
    toSessions,
    { press: mine, say: "Opened" },
    { look: callout, say: "The callout, last in the thread" },
    { hover: inside(callout, role("img", "Nothing left to work on")), say: "The mark, and on hover what it says" },
  ];
}

const prs = inside(callout, role("list", "Pull requests"));
const artifacts = inside(callout, role("list", "Artifacts"));

function keep(narrow: boolean): Step[] {
  const { ledger, opened: inLedger, message } = kit(narrow);
  return [
    ...opened(/Pin the store clock/),
    { look: prs, say: "Each pull request with its state and checks" },
    ...inLedger([{ look: inside(ledger, region("Pull requests")), say: "The ledger's pull requests, with state and checks" }]),
    { press: inside(callout, button("Keep going")), say: "Keep going hides it" },
    { look: message, say: "The cursor is in the message box" },
  ];
}

const made: Step[] = [
  ...opened(/Write up the store clock/),
  { look: artifacts, say: "The page and the Doc it made, and not the picture it looked at" },
];

function end(narrow: boolean): Step[] {
  const { ledger, opened: inLedger } = kit(narrow);
  return [
    ...opened(/Pin the store clock/),
    { press: inside(callout, button("End session")), say: "End session ends the Session" },
    { look: role("img", "Ended"), say: "Ended" },
    ...inLedger([{ look: inside(ledger, region("Pull requests")), say: "Its ledger stays, the pull requests in it" }]),
  ];
}

const wide = walk("session-all-done", keep(false));
const narrow = walk("session-all-done", keep(true), NARROW);
const wideMade = walk("session-all-done-artifacts", made);
const narrowMade = walk("session-all-done-artifacts", made, NARROW);
const wideEnd = walk("session-all-done", end(false));
const narrowEnd = walk("session-all-done", end(true), NARROW);

export {
  wide as "session-all-done",
  narrow as "session-all-done-narrow",
  wideMade as "session-all-done-artifacts",
  narrowMade as "session-all-done-artifacts-narrow",
  wideEnd as "session-all-done-end",
  narrowEnd as "session-all-done-end-narrow",
};
