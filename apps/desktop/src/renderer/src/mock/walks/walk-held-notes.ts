// Notes taken on a Job's walk are held, not sent: the Cockpit stays on the Job it was on, the notes wait as the last entry of
// the Job's thread in the panel beside the page, each editable and removable, and Send notes hands them to the Job as one turn.
// While any are held, Approve asks first. With the panel hidden the bar opens it on them. Sent, the walk stays on screen,
// changing, until its Job shows the page again. A page still loading says so on the bar. Over the `walk-held-notes` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const frame = role("dialog", /^Bridge's window on/);
const panel = region("Job: Retry backoff cap");
const thread = inside(panel, region("Thread"));
const held = inside(thread, region("Pending notes"));
const note = (said: string) => role("listitem", said);
const card = role("dialog", "Capture a note");
const walks = inside(frame, button(/^Walks, /));

/** Capture, a press on the page, the words in the card beside what was picked, and Capture on the card. */
const capture = (said: string, say: string): Step[] => [
  { press: inside(frame, button("Capture", { exact: true })), say: "Capture" },
  { press: button("Pick on the page"), say: "A press picks what is under it" },
  { type: said, into: inside(card, role("textbox", "Note")), say },
  { press: inside(card, button("Capture", { exact: true })), say: "Captured" },
];

export const walkHeldNotes = walk("walk-held-notes", [
  { press: deckRow("Pin the store clock"), say: "The Cockpit is on another Job" },
  { look: region("Job: Pin the store clock"), say: "Pin the store clock is open" },
  { press: inside(frame, button("Capture", { exact: true })), say: "Capture" },
  { press: button("Pick on the page"), say: "A press picks what is under it" },
  { look: inside(card, role("textbox", "Note")), say: "The cursor is in the Note field" },
  { type: "The cap reads 30 s, write thirty seconds", into: inside(card, role("textbox", "Note")), say: "A first note on the backoff table, typed with no press on the field" },
  { press: inside(card, button("Capture", { exact: true })), say: "Captured" },
  { look: inside(frame, text("Note pending in the Job panel")), say: "The bar says the note is pending, not sent" },
  { look: region("Job: Pin the store clock"), say: "The Cockpit stayed on Pin the store clock" },
  { look: inside(held, note("The cap reads 30 s, write thirty seconds")), say: "The note waits at the end of the Job's thread" },
  { look: inside(held, role("img", "Pending")), say: "Marked pending, not sent" },
  ...capture("The last row has no unit", "A second note"),
  { look: inside(frame, text("Notes pending in the Job panel")), say: "Both are pending" },
  { look: inside(held, note("The last row has no unit")), say: "Added under the first, in the same turn" },
  {
    type: "The cap reads 30 s, write 30 seconds in full",
    into: inside(note("The cap reads 30 s, write thirty seconds"), role("textbox", "Note")),
    say: "Its words are edited in place",
  },
  { press: inside(note("The last row has no unit"), button(/^Remove the note/)), say: "Remove takes the second off" },
  { look: inside(held, note("The cap reads 30 s, write 30 seconds in full")), say: "The edit was kept as the field was left" },
  ...capture("The header wraps at this width", "A third note"),
  { press: inside(frame, button(/^Approve/)), say: "Approve asks first, while notes are pending" },
  { look: role("menuitem", "Send notes and approve"), say: "Send them and approve, approve without them, or go back" },
  { press: role("menuitem", "Go back"), say: "Go back approves nothing" },
  { press: inside(frame, button("Hide Job")), say: "Hide Job" },
  { look: inside(frame, text(/^Notes pending$/)), say: "The bar says Notes pending" },
  { press: inside(frame, button("Show notes")), say: "Show notes opens the Job panel" },
  { look: inside(held, note("The header wraps at this width")), say: "The pending notes are in the panel" },
  { press: inside(held, button("Send notes")), say: "Send notes sends every pending note as one turn" },
  // The thread is the one place these words are text: a held note's words are a field's value.
  { look: inside(panel, text(/The cap reads 30 s, write 30 seconds in full/)), say: "The turn is in the Job's thread" },
  { look: inside(panel, text(/The header wraps at this width/)), say: "The third note is in the same turn" },
  { look: inside(frame, button("Walks, Retry backoff cap")), say: "The same walk is on screen" },
  { look: inside(frame, role("img", "Still changing")), say: "Still changing while the Job works on the notes" },
  { press: walks, say: "The walks" },
  { look: role("menuitem", /Pin the store clock/), say: "Two walks are queued" },
  { press: role("menuitem", /Retry backoff cap/), say: "The walk is still in the list" },
  { later: inside(frame, button("Not ready")), say: "Approve is off while the page is changing" },
  { look: inside(frame, role("img", "Ready to walk")), say: "The Job showed the page again: ready to walk" },
  { look: inside(frame, button(/^Approve/)), say: "Approve is back" },
  { look: inside(panel, text(/no longer wraps/)), say: "The Job's reply is in its thread" },
  { press: walks, say: "The walks" },
  { press: role("menuitem", /Pin the store clock/), say: "The other walk" },
  { look: inside(frame, role("img", "Loading")), say: "Its page is loading: the bar says Loading" },
]);
