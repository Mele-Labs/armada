// Each held note shows the whole window as it will be sent, its own box ringed and tagged with its number in the turn, so
// the Job reads note 3 beside the picture tagged 3. Removing a note renumbers the rest. Over the `ringed-note-shots`
// scenario.

import { button, inside, region, role, walk } from "../walk";

const panel = region("Job: Retry backoff cap");
const held = inside(inside(panel, region("Thread")), region("Pending notes"));
const note = (said: string) => inside(held, role("listitem", said));
const SECOND = "The last row has no unit";
const THIRD = "The header wraps at this width";

export const ringedNoteShots = walk("ringed-note-shots", [
  { look: held, say: "Pending notes in the Job panel" },
  { look: inside(note("The cap reads 30 s, write 30 seconds in full"), role("img", /^Window, /)), say: "The whole window as it is sent, the box picked ringed" },
  { look: inside(note(SECOND), role("img", "Note 2")), say: "Each ring is tagged with its note's number in the turn" },
  { press: inside(note(SECOND), button(/^Remove the note/)), say: "Remove the second" },
  { look: inside(note(THIRD), role("img", "Note 2")), say: "The third is now 2" },
]);
