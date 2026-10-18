// A plan task the agent's list marks completed, with no edit seen for it, reads as done. The done check carries a faint mark for the
// missing edit, and the task counts as done in Progress and on the Field. Over the `handed-in-reads-done` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const progress = region("Progress", { exact: true });
const field = region("Field", { exact: true });

export const handedInReadsDone = walk("handed-in-reads-done", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. Progress and the Field share the right column" },
  { look: inside(progress, role("img", "Done, no edit seen")), say: "Progress, with the first group's three tasks done. Two carry a faint mark in the corner: the agent's list says done and no edit was seen for them" },
  { hover: inside(progress, role("img", "Done, no edit seen")), say: "The tooltip names the whole mark: done, no edit seen" },
  { look: inside(field, role("group", "Plan")), say: "The Field draws the same tasks as done with no edit seen, outlined, beside the solid one with edits seen" },
]);
