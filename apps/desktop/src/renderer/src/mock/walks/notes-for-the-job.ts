// Notes the owner leaves for the Job that holds a pull request, on lines of its Files diff. They are kept on his Mac and never posted
// to the forge; Send to job gives the Job every unsent one as a single numbered message, and the notes read as sent. A note the Job resolves folds away until Show resolved. Over `shared-side-panel`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const queue = region("Merge queue");
const pull = role("dialog", "Pull request #1890");
const note = role("textbox", "Note for the job");

const notesForTheJob = walk("shared-side-panel", [
  { press: inside(queue, button(/^#1890/)), say: "A pull request a Job holds" },
  { press: inside(pull, tab("Files")), say: "Files: every file's diff" },
  { press: inside(pull, button(/^Note for the job on \+\s+marker: Option<String>,/)), say: "A press on a line's gutter opens a note for the Job" },
  { type: "Why an Option here, when every pause has a marker?", into: inside(pull, note), say: "Written where the change is" },
  { press: inside(pull, button("Save note")), say: "Kept on this Mac. Nothing is posted to the pull request" },
  { press: inside(pull, button(/^Note for the job on \+\s+at: SystemTime,/)), say: "A second line" },
  { type: "SystemTime can go backwards. Keep Instant and store the offset.", into: inside(pull, note), say: "A second note" },
  { press: inside(pull, button("Save note")), say: "Kept" },
  { look: inside(pull, role("group", "Notes for the job")), say: "Two notes wait, and one press sends them" },
  { press: inside(pull, button("Send to job")), say: "Both go to the Job as one numbered message" },
  { look: inside(pull, role("article", "Note for the job")), say: "The notes stay beside the lines, marked sent" },
  { press: inside(pull, tab("Conversation")), say: "The Job resolves a note with its own tool, and Bridge reads the notes again when Files is entered" },
  { press: inside(pull, tab("Files")), say: "Files again: the note the Job resolved has folded away" },
  { press: inside(pull, button(/^Show resolved \(1\)/)), say: "Show resolved draws it again" },
  { look: inside(pull, text("Resolved by Gate policy on every run")), say: "And which Job resolved it" },
  { press: inside(pull, button(/^Note for the job on -\s+at: Instant,/)), say: "A third line, for a different Job" },
  { type: "Check that nothing else reads this column.", into: inside(pull, note), say: "A third note" },
  { press: inside(pull, button("Save note")), say: "Kept" },
  { press: inside(pull, button("Send to a job")), say: "The caret lists the other running Jobs" },
  { press: inside(role("menu", "Send to a job"), role("menuitem", "Cache key")), say: "Sent to that Job instead of the one holding the pull request" },
]);

export { notesForTheJob as "notes-for-the-job" };
