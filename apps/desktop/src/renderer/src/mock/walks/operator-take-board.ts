// The board take of the Operator: drag a card up and it is taken next, ask for priority in words, put an item waiting on you into Up
// next to talk over, settle it from Discussion, and see it slide to Done. Over `operator-take-board`.

import { button, inside, region, role, text, walk } from "../walk";
import type { Target } from "../walk";

// A dock in a wide window, a sheet in a narrow one.
const panel: Target = {
  ...role("complementary", "Operator", {
    exact: true,
  }),
  or: [
    role("dialog", "Operator", {
      exact: true,
    }),
  ],
};
const body = (name: string): Target =>
  inside(
    panel,
    role("tabpanel", name, {
      exact: true,
    }),
  );
const discussion = body("Discussion");
const now = inside(body("Queue"), region("Now"));
const upNext = inside(body("Queue"), region("Up next"));
const needsYou = inside(body("Queue"), region("Needs you"));
const done = inside(body("Queue"), region("Done"));
const watch = inside(upNext, role("listitem", /^A watch stayed live/));
const legend = inside(upNext, role("listitem", /^Rework the Map legend/));
const doctor = inside(upNext, role("listitem", /^Doctor lists new checks/));
const consoleCard = inside(needsYou, role("listitem", /^Cockpit Console asks/));
const consoleNext = inside(upNext, role("listitem", /^Cockpit Console asks/));
const card = inside(discussion, role("group", /^Item: Cockpit Console/));
const box = inside(discussion, role("textbox", "Message to the Operator"));
const send = inside(discussion, button("Send", { exact: true }));

export const operatorTakeBoard = walk("operator-take-board", [
  {
    key: "Meta+j",
    on: button("Menu", {
      exact: true,
    }),
    say: "⌘J opens the Operator. The board shows its work as it flows: Now, Up next, Needs you and Done",
  },
  {
    drag: watch,
    onto: legend,
    say: "It works down Up next from the top. This one matters more to you, so drag it up. ⌥↑ does the same",
  },
  {
    later: watch,
    say: "It is next",
  },
  {
    look: inside(inside(now, role("listitem", /^A watch stayed live/)), role("img", "The Operator is on this")),
    say: "It finished the card it was on and took yours. Now holds what it is on",
  },
  {
    type: "the doctor checks come first",
    into: box,
    say: "You can also tell it in words",
  },
  {
    press: send,
    say: "Send it",
  },
  {
    look: inside(doctor, text(/^You asked for this first/)),
    say: "It moved the card up and cleared what was holding it, and the card says so",
  },
  {
    press: inside(consoleCard, button(/^More for/)),
    say: "Each card in Needs you has the Operator's pick on its button. You want to talk this one over first",
  },
  {
    press: role("menuitem", /^Discuss when ready/),
    say: "The Operator is busy, so ask to discuss it when it is ready",
  },
  {
    later: consoleNext,
    say: "The card slid into Up next, last in line",
  },
  {
    drag: consoleNext,
    onto: legend,
    say: "You would rather settle it before the Map legend. Drag it up",
  },
  {
    later: consoleNext,
    say: "The Operator finishes what it is on",
  },
  {
    look: card,
    say: "When it gets there, the item comes up in Discussion, pinned above the messages with its button",
  },
  {
    type: "Keep the order, but put the running ones first",
    into: box,
    say: "You want it a little different",
  },
  {
    press: send,
    say: "Send it",
  },
  {
    press: inside(card, button(/^Send: Running bays first/)),
    say: "Its button changed to match. Press it to send that to the Job",
  },
  {
    look: inside(done, role("listitem", /^Cockpit Console asks/)),
    say: "The card slid to Done, saying what was sent, with Open Job beside it",
  },
]);
