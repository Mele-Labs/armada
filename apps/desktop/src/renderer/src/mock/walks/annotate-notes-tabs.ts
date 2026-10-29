// The annotation layer's notes panel, over eight notes on three screens. A search narrows the list by a note's words, its
// screen or the component it is pinned on; tabs split it into open, with no Job, with a Job, and done; a screen's group
// folds; Select all ticks what is listed, and Start job sends the ticked notes to a new Job. The layer is the mock's own
// (`../annotating.tsx`).

import { button, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";

const panel = role("complementary", "All notes");
const bar = role("status");
const head = role("group", "Selected notes");
const search = inside(panel, role("searchbox", "Search notes"));

const steps: Step[] = [
  { press: inside(bar, button(/open, .* done/)), say: "The bar's count opens every note" },
  { look: inside(panel, tab("Open 6")), say: "Open notes first, with how many on each tab" },
  { type: "cut off", into: search, say: "Search by a note's words" },
  { look: inside(panel, text("The repository path is cut off")), say: "Two notes match, on two screens" },
  { type: "RowBadge", into: search, say: "Or by the component it is pinned on" },
  { look: inside(panel, text("The branch chip overlaps the row badge")), say: "The note on RowBadge" },
  { type: "", into: search, say: "Cleared" },
  { press: inside(panel, tab("No job 4")), say: "No job: open notes not sent to a Job" },
  { look: inside(panel, text("The places crowd the search field")), say: "Each one can start a Job" },
  { press: inside(panel, tab("Has job 2")), say: "Has job: open notes already sent" },
  { look: inside(panel, text("Composer draft")), say: "Each names the Job it went to" },
  { press: inside(panel, tab("Done 2")), say: "Done" },
  { look: inside(panel, text("The theme picker has no focus ring")), say: "The finished ones" },
  { press: inside(panel, tab("Open 6")), say: "Back to the open notes" },
  { press: inside(panel, button("Sessions 2")), say: "A screen's name folds its notes away" },
  { look: inside(panel, button("Settings 1")), say: "The other screens stay open" },
  { press: inside(panel, button("Sessions 2")), say: "Pressed again, it unfolds" },
  { look: inside(panel, text("The ledger header wraps under the title")), say: "Its notes are back" },
  { press: inside(panel, tab("No job 4")), say: "No job" },
  { press: inside(head, role("checkbox", "Select all 4")), say: "Select all ticks every note listed" },
  { look: inside(head, text("4 selected")), say: "Four ticked" },
  { press: inside(head, button("Start job", { exact: true })), say: "Start job sends the ticked notes to a new Job" },
  { look: inside(region("Thread"), text("The title is cut off in the row")), say: "The Job opens with the notes as its messages" },
];

const notesTabs = walk("annotate-notes-tabs", steps);

export { notesTabs as "annotate-notes-tabs" };
