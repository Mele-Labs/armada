// Notes taken on a Job's walk are held on the Job, not sent, as they are in the walk window. Capture arms the page, a press picks what is under it, and the card beside the pick holds the words and
// Capture. The notes wait as the last entry of the Job's thread, beside the walk, each editable and removable. While any are held Approve on the bar becomes Send notes, with approving behind its
// caret. They stay held when you leave the Job. Sent, the walk stays, still changing, until the Job shows the page again. A walk tab closed with its × comes back when you press the Job's row. Over the
// `walk-in-job` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";

const billing = role("group", "Walk Billing");
const takes = inside(billing, role("radiogroup", "Pick one"));
const card = role("dialog", "Capture a note");
const held = region("Pending notes");
const note = (said: string) => role("listitem", said);

const FIRST = "The plan card needs the renewal date";
const SECOND = "The invoice list needs a total";

/** Note, a press on the page, the words typed into the card beside the pick, and Capture. */
const capture = (said: string, say: string): Step[] => [
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
    type: said,
    into: inside(card, role("textbox", "Note")),
    say,
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
];

const walkInJobNotes = walk("walk-in-job", [
  {
    press: deckRow("Tidy the account pages"),
    say: "Open the Job. Its thread is beside its walks",
  },
  {
    press: tab("Billing", {
      exact: true,
    }),
    say: "The Billing walk",
  },
  {
    press: inside(takes, role("radio", "Roomy")),
    say: "The Roomy take",
  },
  ...capture(FIRST, "Type the note. The card sits beside what you picked"),
  {
    press: inside(billing, button("Next")),
    say: "The next step",
  },
  {
    press: inside(takes, role("radio", "With table")),
    say: "Another take",
  },
  ...capture(SECOND, "A second note, on another part"),
  {
    look: inside(held, note(FIRST)),
    say: "The notes wait at the end of the thread, each naming its walk and take",
  },
  {
    look: inside(
      billing,
      button("Send notes", {
        exact: true,
      }),
    ),
    say: "Approve on the bar has become Send notes",
  },
  {
    press: inside(billing, button("Approve options")),
    say: "Behind the caret you can send the notes and approve With table, or approve it without them",
  },
  {
    press: inside(billing, button("Approve options")),
    say: "Close it. Nothing was sent or approved",
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
    look: inside(held, note(FIRST)),
    say: "The notes are still held",
  },
  {
    press: inside(
      billing,
      button("Send notes", {
        exact: true,
      }),
    ),
    say: "Send notes on the bar sends what is held as one turn",
  },
  {
    look: inside(region("Pane: Thread"), text(/^The plan card needs the renewal date/)),
    say: "The turn is in the thread, and the pending notes are gone",
  },
  {
    look: inside(billing, button(/^Not ready$/)),
    say: "The walk stays, still changing, and Approve reads Not ready",
  },
  {
    later: inside(billing, button(/^Not ready$/)),
    say: "Time passes while the Job works on the notes",
  },
  {
    look: inside(billing, role("img", "Ready to walk")),
    say: "The Job showed the page again, ready to walk",
  },
  {
    press: tab("Settings", {
      exact: true,
    }),
    say: "The Settings walk",
  },
  {
    press: button("Close Settings"),
    say: "Close the tab with its ×. The walk is still owed and the row still pulses",
  },
  {
    press: deckRow("Tidy the account pages"),
    say: "Pressing the row brings the tab back",
  },
]);

export { walkInJobNotes as "walk-in-job-notes" };
