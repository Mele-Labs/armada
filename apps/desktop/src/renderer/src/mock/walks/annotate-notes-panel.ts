// Every note the owner has left, across screens, in a panel on the layer. The bar's list button opens
// it; notes are grouped by the screen they were left on, open first. One note goes to a Job from its
// row, another to a live Session. Told twice, wide and below the breakpoint. The layer is the mock's
// own (`../annotating.tsx`).

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { thread } = kit(narrow);
  const panel = role("complementary", "All notes");
  const bar = role("status");
  const ledger = inside(panel, text("The ledger header wraps under the title"));
  return [
    { press: inside(bar, button(/open, .* done/)), say: "The bar's count opens every note" },
    { look: inside(panel, text("Sessions")), say: "Grouped by the screen each was left on" },
    { look: inside(panel, text("No screen")), say: "A note with none has its own group" },
    { look: inside(panel, text("49-the-repo-field")), say: "A note already sent shows where it went" },
    { press: inside(panel, button("Dispatch job")), say: "A row dispatches its one note as a Job" },
    { look: inside(panel, text("57-the-note-job")), say: "The row then shows the Job" },
    { look: ledger, say: "Another row, on another screen" },
    { press: inside(panel, button("Send to a Session")), say: "Its caret lists the live Sessions" },
    { press: role("menuitem", "Release notes script"), say: "A pick sends that one note to the Session" },
    { look: inside(thread, text("The ledger header wraps under the title")), say: "The Session opens with the note as its message" },
  ] satisfies Step[];
}

const wide = walk("annotate-to-session", steps(false));
const narrow = walk("annotate-to-session", steps(true), NARROW);

export { wide as "annotate-notes-panel", narrow as "annotate-notes-panel-narrow" };
