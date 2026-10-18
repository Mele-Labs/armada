// A note just saved stays up on the page, and its card has the panel row's acts: Start
// session with its caret, Mark done and Delete. A note sent to a Session opens that Session on the
// Cockpit, not on the Sessions page. Told twice, wide and below the breakpoint. The layer is the mock's
// own (`../annotating.tsx`).

import { kit, NARROW } from "../sessions/walk-kit";
import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { thread } = kit(narrow);
  const card = dialog("Note");
  return [
    { press: deckRow("Fix the flaky store test"), say: "A press on the page starts a note" },
    { type: "The row should say which step it is on", into: inside(dialog("New note"), role("textbox", "Note")), say: "What is wrong with it" },
    { press: button("Save note"), say: "Saved" },
    { look: inside(card, text("The row should say which step it is on")), say: "The note stays up where it was left" },
    { look: inside(card, button("Mark done")), say: "With the acts the notes panel has: Mark done and Delete" },
    { press: inside(card, button("Send to a Job")), say: "The caret lists the live Jobs" },
    { press: role("menuitem", "Fix the flaky store test"), say: "A pick sends the note to that Job" },
    { look: region("Job: Fix the flaky store test"), say: "The Job opens on the Cockpit" },
    { look: inside(thread, text("The row should say which step it is on")), say: "With the note as its message" },
  ] satisfies Step[];
}

const wide = walk("annotate-to-session", steps(false));
const narrow = walk("annotate-to-session", steps(true), NARROW);

export { wide as "annotate-card-acts", narrow as "annotate-card-acts-narrow" };
