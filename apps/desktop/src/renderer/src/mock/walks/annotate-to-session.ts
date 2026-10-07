// Notes left with the annotation layer, sent to a Session. The bar offers Dispatch job, which
// proposes the notes as a Job, and Start session, which sends them as the first message of a new
// Session. Its caret lists every live Session, hosted or from a terminal. Told twice, wide and
// below the breakpoint. The layer is the mock's own (`../annotating.tsx`), with a sink that sends.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { thread } = kit(narrow);
  const bar = role("status");
  return [
    { look: inside(bar, button("Dispatch job")), say: "The bar sends the notes as a Job" },
    { look: inside(bar, button("Start session")), say: "Or as a message to a Session" },
    { press: inside(bar, button("Send to a Session")), say: "The caret lists the live Sessions" },
    { look: role("menuitem", "Fix the flaky store test"), say: "One Bridge hosts" },
    { look: role("menuitem", "Release notes script"), say: "And one from a terminal" },
    { press: role("menuitem", "Release notes script"), say: "A pick sends the notes to that Session" },
    { look: inside(thread, text("The title is cut off in the row")), say: "The Session opens with the notes as its message" },
  ] satisfies Step[];
}

const wide = walk("annotate-to-session", steps(false));
const narrow = walk("annotate-to-session", steps(true), NARROW);

export { wide as "annotate-to-session", narrow as "annotate-to-session-narrow" };
