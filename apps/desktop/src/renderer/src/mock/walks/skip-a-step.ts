// Skipping a step of a Job's workflow, and undoing it. The Job is on the feature workflow at Walk, with Refine and Deliver to come: Refine is skipped from its
// band's right-press menu, Walk from its icon, which moves the Job past both to Deliver; then each skip is undone. Over the `skip-a-step` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const at = region("Walk", { exact: true });
const refine = region("Refine", { exact: true });
const deliver = region("Deliver", { exact: true });

export const skipAStep = walk("skip-a-step", [
  { press: deckRow("Rotate the audit log"), say: "Open a Job on the feature workflow. It is at Walk, with Refine and Deliver to come" },
  { hover: inside(refine, button("Skip step", { exact: true })), say: "A step to come has a Skip icon in its band. Deliver has none: it is where the work lands" },
  { context: inside(refine, role("heading", "Refine", { exact: true })), say: "A right press on a step's band opens its menu" },
  { press: role("menuitem", "Skip step", { exact: true }), say: "Skip Refine" },
  { look: inside(refine, text("Skipped")), say: "Refine stays on the spine, faint and struck, with Skipped and an Undo. The Job will pass over it" },
  { press: inside(at, button("Skip step", { exact: true })), say: "Skip the current step from its icon" },
  { look: deliver, say: "The Job moves past Walk and Refine, both skipped, to Deliver" },
  { press: inside(at, button("Undo skip", { exact: true })), say: "Undo puts the Job back at Walk" },
  { context: inside(refine, role("heading", "Refine", { exact: true })), say: "Refine's menu now offers the undo" },
  { press: role("menuitem", "Undo skip", { exact: true }), say: "Undo skip" },
  { look: refine, say: "Refine is a step to come again" },
]);
