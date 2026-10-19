// Notes taken on a Job's walk are held, not sent: the Cockpit stays on the Job it was on, the notes wait in the Job panel
// beside the page, each editable and removable, and Send notes hands them to the Job as one turn. While any are held,
// Approve asks first. Over the `walk-held-notes` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const frame = role("dialog", /^Bridge's window on/);
const panel = region("Job: Retry backoff cap");
const held = region("Held notes");
const note = (said: string) => role("listitem", said);
const form = role("form", "Note on what you picked");

/** Capture, a press on the page, the words, and Capture on the note. */
const capture = (said: string, say: string): Step[] => [
  { press: inside(frame, button("Capture", { exact: true })), say: "Capture" },
  { press: button("Pick on the page"), say: "A press picks what is under it" },
  { type: said, into: inside(form, role("textbox", "What is wrong here")), say },
  { press: inside(form, button("Capture", { exact: true })), say: "Captured" },
];

export const walkHeldNotes = walk("walk-held-notes", [
  { press: deckRow("Pin the store clock"), say: "The Cockpit is on another Job" },
  { look: region("Job: Pin the store clock"), say: "Pin the store clock is open" },
  ...capture("The cap reads 30 s, write thirty seconds", "A first note on the backoff table"),
  { look: inside(frame, text("Note held in the Job panel")), say: "The bar says the note is held, not sent" },
  { look: region("Job: Pin the store clock"), say: "The Cockpit stayed on Pin the store clock" },
  { look: inside(held, note("The cap reads 30 s, write thirty seconds")), say: "The note waits in the Job panel" },
  ...capture("The last row has no unit", "A second note"),
  { look: inside(frame, text("Notes held in the Job panel")), say: "Both are held" },
  { look: inside(held, note("The last row has no unit")), say: "Listed under the first" },
  {
    type: "The cap reads 30 s, write 30 seconds in full",
    into: inside(note("The cap reads 30 s, write thirty seconds"), role("textbox", "Note")),
    say: "Its words are edited in place",
  },
  { press: inside(note("The last row has no unit"), button(/^Remove the note/)), say: "Remove takes the second off" },
  { look: inside(held, note("The cap reads 30 s, write 30 seconds in full")), say: "The edit was kept as the field was left" },
  ...capture("The header wraps at this width", "A third note"),
  { press: inside(frame, button(/^Approve/)), say: "Approve asks first, while notes are held" },
  { look: role("menuitem", "Send notes and approve"), say: "Send them and approve, approve without them, or go back" },
  { press: role("menuitem", "Go back"), say: "Go back approves nothing" },
  { press: inside(held, button("Send notes")), say: "Send notes sends every held note as one turn" },
  // The thread is the one place these words are text: a held note's words are a field's value.
  { look: inside(panel, text(/The cap reads 30 s, write 30 seconds in full/)), say: "The turn is in the Job's thread" },
  { look: inside(panel, text(/The header wraps at this width/)), say: "The third note is in the same turn" },
]);
