// The board take of the Operator: drag a card up and it is taken next, ask for priority in words, bring an item waiting on you up in
// Discussion, settle it there, and see it slide to Done. Over `operator-take-board`.

import { button, inside, region, role, text, walk } from "../walk";
import type { Target } from "../walk";

// The Operator's own page.
const panel: Target = role("region", "Operator", {
  exact: true,
});
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
const doctor = inside(upNext, role("listitem", /^Doctor lists new checks/));
const consoleCard = inside(needsYou, role("listitem", /^Cockpit Console/));
const card = inside(discussion, role("group", /^Discussing: Cockpit Console/));
const box = inside(discussion, role("textbox", "Message to the Operator"));
const send = inside(
  discussion,
  button("Send", {
    exact: true,
  }),
);

export const operatorTakeBoard = walk("operator-take-board", [
  {
    key: "Meta+j",
    on: button("Menu", {
      exact: true,
    }),
    say: "⌘J goes to the Operator, a page of its own. The board shows its work as it flows: Now, Up next, Needs you and Done",
  },
  {
    drag: watch,
    onto: inside(now, role("listitem", /^Time each build step/)),
    say: "It works down Up next from the top. This one matters more to you: drop it on the card in Now and it is taken next. ⌥↑ moves a card up one place",
  },
  {
    later: watch,
    say: "It is next",
  },
  {
    look: inside(
      inside(now, role("listitem", /^A watch stayed live/)),
      role("img", "The Operator is on this"),
    ),
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
    press: role("menuitem", /^Discuss now/),
    say: "Bring it up in Discussion",
  },
  {
    look: card,
    say: "It comes up in Discussion at once, with the Operator's pick and the other ways on its buttons",
  },
  {
    press: inside(card, button(/^Go with Keep the order/)),
    say: "Go with what it recommends",
  },
  {
    look: inside(done, role("listitem", /^Cockpit Console/)),
    say: "The card slid to Done, saying what was sent and who decided it, with Open Job beside it",
  },
]);
