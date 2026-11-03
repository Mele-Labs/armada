// A Job's walk is a tab of the Job, and no window opens. The Job's row pulses while it owes you a walk, and each tab pulses until it has been in front of you. The bar is two rows on every walk: the first has
// the takes under Pick one, then Reload, Capture and Approve, and the second has Back, the step's count, Next and the line. Billing has three takes by name and Approve names the one on screen; Settings has
// none. A note is held on the Job, not sent, and waits at the end of the Job's thread, and while any is held Approve becomes Send notes, with approving behind its caret. Sent, the walk stays, still changing,
// until the Job shows the page again. Leaving for another Job and coming back finds the page, take, step and notes as you left them, a tab closed with its × comes back, and approving closes the tab. ← and →
// walk the takes, Tab reaches every control and ⌘↵ presses Approve or Send notes. Over the `walk-in-job` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";

const billing = role("group", "Walk Billing");
const settings = role("group", "Walk Settings");
const takes = inside(billing, role("radiogroup", "Pick one"));
const card = role("dialog", "Capture a note");
const held = region("Pending notes");
const sendNotes = inside(
  billing,
  button("Send notes", {
    exact: true,
  }),
);

const NOTE = "The plan card needs the renewal date";

const walkInJob = walk("walk-in-job", [
  {
    press: deckRow("Fold Jobs into Sessions"),
    say: "A Job with no walk is open. No window opens",
  },
  {
    look: deckRow("Tidy the account pages"),
    say: "The row of a Job with a walk pulses",
  },
  {
    press: deckRow("Tidy the account pages"),
    say: "The thread is at the left and the walks are tabs at the right",
  },
  {
    look: tab("Settings", {
      exact: true,
    }),
    say: "Billing is in front and has stopped pulsing. Settings still pulses",
  },
  {
    look: inside(billing, text("Pick one")),
    say: "Pick one lists the takes, and the filled mark is the one on screen",
  },
  {
    press: inside(takes, role("radio", "Roomy")),
    say: "A take swaps with one press",
  },
  {
    look: inside(billing, button(/^Approve Roomy$/)),
    say: "Approve names the take on screen",
  },
  {
    press: inside(
      billing,
      button("Capture", {
        exact: true,
      }),
    ),
    say: "Capture on the bar arms the page",
  },
  {
    press: button("Pick on the page"),
    say: "A press picks what is under it",
  },
  {
    type: NOTE,
    into: inside(card, role("textbox", "Note")),
    say: "Type the note. The card sits beside what you picked",
  },
  {
    press: inside(
      card,
      button("Capture", {
        exact: true,
      }),
    ),
    say: "Capture holds the note on the Job",
  },
  {
    look: inside(held, role("listitem", NOTE)),
    say: "The note waits at the end of the thread, naming its walk and take",
  },
  {
    look: sendNotes,
    say: "Approve on the bar has become Send notes",
  },
  {
    press: inside(billing, button("Approve options")),
    say: "Behind the caret you can send the notes and approve Roomy, or approve it without them",
  },
  {
    press: inside(billing, button("Approve options")),
    say: "Close it. Nothing was sent or approved",
  },
  {
    press: inside(billing, button("Next")),
    say: "Next goes to the next step",
  },
  {
    press: tab("Settings", {
      exact: true,
    }),
    say: "Settings has no takes, so Pick one is not there, and its bar is the same two rows",
  },
  {
    press: button("Close Settings"),
    say: "Close the tab with its ×. The walk is still owed and the row still pulses",
  },
  {
    press: deckRow("Pin the store clock"),
    say: "Another Job",
  },
  {
    press: deckRow("Tidy the account pages"),
    say: "Back in the first Job. Pressing the row brought Settings back",
  },
  {
    press: tab("Billing", {
      exact: true,
    }),
    say: "Billing",
  },
  {
    look: inside(billing, text("2 of 5")),
    say: "The take, the step and the note are as you left them",
  },
  {
    press: sendNotes,
    say: "Send notes sends what is held as one turn",
  },
  {
    look: inside(region("Pane: Thread"), text(/^The plan card needs the renewal date/)),
    say: "The turn is in the thread, and the pending note is gone",
  },
  {
    look: inside(billing, button(/^Not ready$/)),
    say: "The walk stays, still changing, and Approve reads Not ready",
  },
  {
    later: inside(billing, button(/^Not ready$/)),
    say: "Time passes while the Job works on the note",
  },
  {
    look: inside(billing, role("img", "Ready to walk")),
    say: "The Job showed the page again, ready to walk",
  },
  {
    press: inside(billing, button(/^Approve Roomy$/)),
    say: "Approving Billing closes its tab",
  },
  {
    press: inside(
      settings,
      button("Approve", {
        exact: true,
      }),
    ),
    say: "Approving Settings closes the last tab, and the row stops pulsing",
  },
  {
    press: deckRow("Pin the store clock"),
    say: "The next Job with a walk",
  },
]);

export { walkInJob as "walk-in-job" };
