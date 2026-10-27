// A plan task the agent's list marks completed, with no edit seen for it, reads as done. The done check carries a faint mark for the
// missing edit, and the task counts as done in the Now zone and on the Field. Over the `handed-in-reads-done` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const now = region("Now", { exact: true });
const field = region("Field", { exact: true });
const large = role("group", "Expanded Field");

export const handedInReadsDone = walk("handed-in-reads-done", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. Its story is in the right column, and the Field is under the Jobs list" },
  { press: inside(now, button("Whole plan")), say: "Now shows the group being worked. One press opens the whole plan, a glyph for every task" },
  { look: inside(now, role("img", "Done, no edit seen")), say: "The whole plan, with the first group's three tasks done. Two carry a faint mark in the corner: the agent's list says done and no edit was seen for them" },
  { hover: inside(now, role("img", "Done, no edit seen")), say: "The tooltip names the whole mark: done, no edit seen" },
  { press: inside(field, button("Expand")), say: "Expand opens the Field large, with the plan along its foot" },
  { look: inside(large, role("group", "Plan")), say: "The Field draws the same tasks as done with no edit seen, outlined, beside the solid one with edits seen" },
]);
