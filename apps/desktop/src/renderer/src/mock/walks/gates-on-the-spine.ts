// A step's gate as a stage on the spine, with a face of its own per kind
// (prototype, 5 Oct 2026). The owner's reading: the next step cannot start
// until the gate passes, so hanging it beside the step made it look optional.
// Staged — four steps at their gates at once, not one Job's moment.

import { button, card, inside, region, role, text, walk } from "../walk";

const RUN = region("This Job's run");

export const gatesOnTheSpine = walk("proto/gates-on-the-spine", [
  { look: RUN, say: "Step, then its gate, then the next step: nothing hangs beside" },
  { look: role("button", /^Judge, advanced/), say: "A gate that passed" },
  { look: inside(role("button", /^Checks, running/), text("test screens::approval_life")), say: "Checks running: the commands, and the last line one printed" },
  { look: inside(role("button", /^Checks, running/), text("typecheck")), say: "One failed already: the failure state's glyph and colour" },
  { look: inside(card("Implement"), text("attempt 2")), say: "The gate sends Implement back: the loop returns to it on the right edge, labelled attempt 2 of 3" },
  { look: inside(role("button", /^Judge, running/), role("img", /^Judge 3, /)), say: "A Judge part-way through its panel: two landed, one still reading" },
  { look: role("button", /^You, needs review/), say: "A human gate, held, and how long it has waited" },
  { press: role("button", /^Checks, running/), say: "A gate stage opens the step's own panel on a running Job" },
  { look: button("Back to Overview"), say: "with the way back" },
]);
