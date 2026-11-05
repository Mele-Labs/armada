// One moment with the Operator: put what matters first and it takes that next, then ask to discuss an item when it is ready and settle it
// from its card in the conversation. Over `operator-queue`.

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
const queue = inside(body("Queue"), region("Operator's Queue"));
const waiting = inside(body("Queue"), region("Waiting on you"));
const watch = inside(queue, role("listitem", /^A watch stayed live/));
const legend = inside(queue, role("listitem", /^Rework the Map legend/));
const doctor = inside(queue, role("listitem", /^Doctor lists new checks/));
const toDiscuss = inside(queue, role("listitem", /^Cockpit Console asks/));
const consoleRow = inside(waiting, role("listitem", /^Cockpit Console asks/));
const card = inside(discussion, role("group", /^Item: Cockpit Console/));
const box = inside(discussion, role("textbox", "Message to the Operator"));
const send = inside(discussion, button("Send", { exact: true }));

export const operatorQueue = walk("operator-queue", [
  {
    key: "Meta+j",
    on: button("Menu", {
      exact: true,
    }),
    say: "⌘J opens the Operator: your Discussion with it, and its Queue",
  },
  {
    drag: watch,
    onto: legend,
    say: "It works down its queue from the top. This row matters more to you, so drag it up. ⌥↑ does the same",
  },
  {
    later: inside(watch, text(/^You dispatched it/)),
    say: "It is second now, so it is what the Operator takes next",
  },
  {
    look: inside(watch, role("img", "The Operator is reading this")),
    say: "It finished the row it was on and took yours. The row it is reading is ruled in blue",
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
    look: doctor,
    say: "It acts right away: the row is next, and it cleared what was holding it. It can do that because Delegate is On",
  },
  {
    look: inside(consoleRow, button(/^Send: Keep the order/)),
    say: "Each row in Waiting on you has the Operator's pick on its button, so one press does what it would do",
  },
  {
    press: inside(consoleRow, button(/^More for/)),
    say: "The rest are behind the arrow. You want to talk this one over first",
  },
  {
    press: role("menuitem", /^Discuss when ready/),
    say: "The Operator is busy, so ask to discuss it when it is ready",
  },
  {
    drag: toDiscuss,
    onto: doctor,
    say: "So it joins the Operator's Queue, and you can place it like any other row",
  },
  {
    later: toDiscuss,
    say: "It is next in the Operator's Queue",
  },
  {
    look: card,
    say: "When the Operator gets to it, it comes up in Discussion, pinned above the messages with its button. Talk never buries it",
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
    look: inside(consoleRow, text(/^Sent: Keep slot order/)),
    say: "The row and the card both say what came of it, with Open Job beside it",
  },
]);
