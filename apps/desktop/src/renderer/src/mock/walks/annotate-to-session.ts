// Notes left with the annotation layer, sent to a Session. The bar offers Start job, which
// sends them as the first message of a new Session. Its caret lists every live Session, hosted or from a terminal. Told twice, wide and
// below the breakpoint. A start Fleet refuses is a toast. The layer is the mock's own (`../annotating.tsx`), with a sink that sends.

import { kit, NARROW } from "../sessions/walk-kit";
import { button, inside, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { thread } = kit(narrow);
  const bar = role("status");
  return [
    { look: inside(bar, button("Start job")), say: "The bar sends the notes as a message to a Job" },
    { press: inside(bar, button("Start job")), say: "A start Fleet refuses is a toast with its reason, and the notes stay unsent" },
    { look: text("Job not started: Fleet refused it"), say: "Nothing is written on the bar" },
    { press: inside(bar, button("Send to a Job")), say: "The caret lists the live Jobs" },
    { look: role("menuitem", "Fix the flaky store test"), say: "One Bridge hosts" },
    { look: role("menuitem", "Release notes script"), say: "And one from a terminal" },
    { press: role("menuitem", "Release notes script"), say: "A pick sends the notes to that Job" },
    { look: inside(thread, text("The title is cut off in the row")), say: "The Job opens with the notes as its message" },
  ] satisfies Step[];
}

const wide = walk("annotate-to-session", steps(false));
const narrow = walk("annotate-to-session", steps(true), NARROW);

export { wide as "annotate-to-session", narrow as "annotate-to-session-narrow" };
