// A Job's walk is a tab of the Job, and no window opens. The Job's row pulses while it owes you a walk, and each tab pulses until it has been in front of you. The bar is two rows on every walk: the first has
// the takes under Pick one, then Reload, Capture and Approve, and the second has Back, the step's count, Next and the line. Billing has three takes by name and Approve names the one on screen; Settings has
// none. Leaving for another Job and coming back finds the page, take and step as you left them, and approving closes the tab. ← and → walk the takes, Tab reaches every control and ⌘↵ approves. Over the
// `walk-in-job` scenario.

import { button, deckRow, inside, role, tab, text, walk } from "../walk";

const billing = role("group", "Walk Billing");
const settings = role("group", "Walk Settings");
const takes = inside(billing, role("radiogroup", "Pick one"));

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
    press: inside(billing, button("Next")),
    say: "Next goes to the next step",
  },
  {
    look: inside(billing, text("2 of 5")),
    say: "The step count moved",
  },
  {
    press: tab("Settings", {
      exact: true,
    }),
    say: "Settings has no takes, so Pick one is not there, and its bar is the same two rows",
  },
  {
    look: inside(
      settings,
      button("Approve", {
        exact: true,
      }),
    ),
    say: "Approve is at the end of the first row, where it is on every walk",
  },
  {
    press: deckRow("Pin the store clock"),
    say: "Another Job",
  },
  {
    press: deckRow("Tidy the account pages"),
    say: "Back in the first Job",
  },
  {
    press: tab("Billing", {
      exact: true,
    }),
    say: "Billing",
  },
  {
    look: inside(billing, button(/^Approve Roomy$/)),
    say: "The take is the one you left",
  },
  {
    look: inside(billing, text("2 of 5")),
    say: "So is the step",
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
