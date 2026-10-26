// A Job's document. An investigation writes a page of its own design, with a live sketch in it, and the Landing's report row opens it in a
// layer over the whole Job: contents, the page, the Notes margin. Words, a part of the sketch and a row of the table are noted and sent
// as one message; the Job answers with revision 2, where Changes marks what moved on and each note shows the change that answered it.
// Over `job-story`, at the report. The page and its second revision are `mock/document-pages.ts`.

import { LEASE_DOC_TITLE } from "../document-pages";
import { button, deckRow, inside, region, role, row, text, walk } from "../walk";

const blueprint = region("Blueprint", { exact: true });
const landing = region("Landing", { exact: true });
const layer = region("Document", { exact: true });
const thread = region("Thread");
const page = inside(layer, role("article", LEASE_DOC_TITLE, { exact: true }));
const contents = inside(layer, role("navigation", "Contents"));
const margin = inside(layer, role("complementary", "Notes on the page"));
const report = inside(landing, button(LEASE_DOC_TITLE));
const add = role("menuitem", "Add note");
const note = role("textbox", "Note");
const measured = inside(contents, button("What was measured"));

const reading = walk("job-story", [
  { press: deckRow(LEASE_DOC_TITLE), say: "An investigation whose plan has just been recorded" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { later: landing, say: "The page is written" },
  { look: report, say: "Landing shows its title and the first lines of its conclusion" },
  { press: report, say: "A press opens the page in a layer over the whole Job" },
  { look: layer, say: "Contents on the left, the page in the middle, the Notes margin on the right" },
  { press: measured, say: "A press in the contents scrolls to its section" },
  { key: "ArrowUp", on: measured, say: "The arrows walk the contents" },
  { select: "leave the limit as it is", in: page, say: "Words selected" },
  { key: "Shift+F10", on: page, say: "Shift F10, or the menu key, opens the menu at the selection" },
  { press: add, say: "Add note" },
  { type: "Why not raise the limit instead?\n", into: note, say: "Enter holds the note" },
  { look: inside(page, role("img", "Note 1")), say: "Its number stands at the words" },
  { context: inside(page, role("group", "Check runs", { exact: true })), say: "Right-click a part of the sketch" },
  { press: add, say: "Add note" },
  { type: "Does the Drone do anything while the Check runs?\n", into: note, say: "A second note" },
  { look: inside(inside(page, role("group", "Check runs", { exact: true })), role("img", "Note 2")), say: "Its number stands on the part" },
  { context: inside(page, row("Rename the bay tile")), say: "Right-click a row of the table" },
  { press: add, say: "Add note" },
  { type: "What expired this one?\n", into: note, say: "A third note" },
  { look: inside(margin, role("list", "Notes held")), say: "The margin lists what is held" },
  { key: "Meta+Enter", on: page, say: "⌘Enter sends all three as one message" },
  { look: inside(margin, role("list", "Notes sent")), say: "They stand in the margin as sent" },
  { key: "Escape", on: layer, say: "Esc" },
  { look: landing, say: "The Job is back as it was" },
  { look: inside(thread, role("list", "Notes")), say: "One numbered message in the Job's thread" },
  { press: inside(thread, button(/leave the limit as it is/)), say: "A note's place in the thread opens the page there" },
  { look: inside(page, text("leave the limit as it is")), say: "At the words it was about" },
  { key: "Escape", on: layer, say: "Esc" },
  { later: landing, say: "The Job writes revision 2" },
  { press: report, say: "The page again" },
  { look: inside(layer, button(/^Revision 2/)), say: "Revision 2" },
  { key: "c", on: layer, say: "C turns Changes on" },
  { look: inside(page, text("The limit stays because it is what frees a bay when a Drone dies mid turn")), say: "Words that came are marked, words that went are struck" },
  { press: inside(margin, button(/^Added in revision 2/)), say: "Each note shows the change that answered it, and a press goes there" },
  { look: inside(page, text("The clock does not stop while a Check runs")), say: "The block that answers the sketch note" },
  { key: "r", on: layer, say: "R looks at revision 1 again" },
  { look: inside(layer, button(/^Revision 1/)), say: "Revision 1, as it was sent" },
  { key: "r", on: layer, say: "R again, and the newest is back" },
  { key: "s", on: layer, say: "S opens Share" },
  { look: role("menuitem", "Save as PDF"), say: "Save as web page, Save as PDF, Add to the repo" },
  { press: role("menuitem", "Add to the repo"), say: "Add to the repo asks the Job to commit the page" },
  { look: inside(thread, text("to the repo.")), say: "The Job is asked in its thread" },
  { key: "Escape", on: layer, say: "Esc" },
  { look: landing, say: "The Job is back as it was" },
]);

export { reading as "job-document" };
