// A Job's document. An investigation writes a page of its own design, with a live sketch in it, and Completed's report row opens it in a
// floating panel over the Job: Contents with the Notes under it at the left, the page beside them. Words, a part of the sketch and a row
// of the table are noted and sent as one message; the Job answers with revision 2, where Changes marks what moved on and each note shows
// the change that answered it.
// Over `job-story`, at the report. The page and its second revision are `mock/document-pages.ts`.

import { LEASE_DOC_TITLE } from "../document-pages";
import { button, deckRow, inside, region, role, row, text, walk } from "../walk";

const now = region("Now", { exact: true });
const landing = region("Completed", { exact: true });
const layer = region("Document", { exact: true });
// The panel the document is drawn in, named by its title: its head holds the revision menu and the tools.
const sheet = role("dialog", LEASE_DOC_TITLE, { exact: true });
const thread = region("Thread");
const page = inside(layer, role("article", LEASE_DOC_TITLE, { exact: true }));
const contents = inside(layer, role("navigation", "Contents"));
const notes = inside(layer, role("complementary", "Notes on the page"));
const report = inside(landing, button(LEASE_DOC_TITLE));
const add = role("menuitem", "Add note");
const note = role("textbox", "Note");
const measured = inside(contents, button("What was measured"));
const clock = inside(contents, button("How the lease clock runs"));

const reading = walk("job-story", [
  { press: deckRow(LEASE_DOC_TITLE), say: "An investigation whose plan has just been recorded" },
  { press: inside(now, button("Approve")), say: "Approve" },
  { later: landing, say: "The page is written" },
  { look: report, say: "Completed shows its title and the first lines of its conclusion" },
  { press: report, say: "A press opens the page in a panel over the Job, the Job dimmed behind it" },
  { look: layer, say: "Contents on the left with the Notes under it, the page beside them" },
  { look: inside(sheet, button(/^Revision 1/)), say: "The revision is a menu under the title from the first" },
  { select: "leave the limit as it is", in: page, say: "Words selected" },
  { key: "Shift+F10", on: page, say: "Shift F10, or the menu key, opens the menu at the selection" },
  { press: add, say: "Add note" },
  { type: "Why not raise the limit instead?\n", into: note, say: "Enter holds the note" },
  { look: inside(page, role("img", "Note 1")), say: "Its number stands at the words" },
  { press: clock, say: "A press in the contents scrolls to its section" },
  { context: inside(page, role("group", "Check runs", { exact: true })), say: "Right-click a part of the sketch" },
  { press: add, say: "Add note" },
  { type: "Does the Drone do anything while the Check runs?\n", into: note, say: "A second note" },
  { look: inside(inside(page, role("group", "Check runs", { exact: true })), role("img", "Note 2")), say: "Its number stands on the part" },
  { press: measured, say: "The next section" },
  { key: "ArrowDown", on: measured, say: "The arrows walk the contents" },
  { hover: inside(page, row("Pin the store clock")), say: "Rows ruled and shaded alternately; the row under the pointer tints" },
  { context: inside(page, row("Rename the bay tile")), say: "Right-click a row of the table" },
  { press: add, say: "Add note" },
  { type: "What expired this one?\n", into: note, say: "A third note" },
  { look: inside(notes, role("list", "Notes held")), say: "The Notes list what is held" },
  { hover: inside(notes, button(/Why not raise the limit/)), say: "A note under the pointer focuses its words in the page" },
  { key: "Meta+Enter", on: page, say: "Send to Job, or ⌘Enter, sends all three as one message" },
  { look: inside(notes, role("list", "Notes sent")), say: "They stand in the Notes as sent, and highlighted in the page" },
  { hover: inside(page, row("Rename the bay tile")), say: "A row or words under the pointer light their note" },
  { key: "m", on: layer, say: "M hides the Notes" },
  { look: page, say: "The page widens, and Contents stays where it was" },
  { key: "m", on: layer, say: "M shows it again" },
  { look: inside(page, text("Compare the expiries")), say: "A list shows its bullets or numbers, indented under the text, a nested one with circles" },
  { key: "Escape", on: layer, say: "Esc" },
  { look: landing, say: "The Job is back as it was" },
  { look: inside(thread, role("list", "Notes")), say: "One numbered message in the Job's thread" },
  { press: inside(thread, button(/leave the limit as it is/)), say: "A note's place in the thread opens the page there" },
  { look: inside(page, text("leave the limit as it is")), say: "At the words it was about" },
  { key: "Escape", on: layer, say: "Esc" },
  { later: landing, say: "The Job writes revision 2" },
  { press: report, say: "The page again" },
  { look: inside(sheet, button(/^Revision 2/)), say: "Revision 2" },
  { key: "c", on: layer, say: "C turns Changes on" },
  { look: inside(page, text("The limit stays because it is what frees a bay when a Drone dies mid turn")), say: "Words that came are marked, words that went are struck" },
  { press: inside(notes, button(/^Added in revision 2/)), say: "Each note shows the change that answered it, and a press goes there" },
  { look: inside(page, text("The clock does not stop while a Check runs")), say: "The block that answers the sketch note" },
  { press: inside(sheet, button(/^Revision 2/)), say: "Every revision is listed, newest first" },
  { press: role("menuitem", "Revision 1"), say: "Revision 1" },
  { look: inside(sheet, button(/^Revision 1/)), say: "As it was sent" },
  { key: "r", on: layer, say: "R brings the newest back" },
  { key: "s", on: layer, say: "S opens Share" },
  { look: role("menuitem", "Save as PDF"), say: "Save as web page, Save as PDF, Add to the repo" },
  { press: role("menuitem", "Add to the repo"), say: "Add to the repo asks the Job to commit the page" },
  { look: inside(thread, text("to the repo.")), say: "The Job is asked in its thread" },
  { key: "Escape", on: layer, say: "Esc" },
  { look: landing, say: "The Job is back as it was" },
]);

export { reading as "job-document" };
