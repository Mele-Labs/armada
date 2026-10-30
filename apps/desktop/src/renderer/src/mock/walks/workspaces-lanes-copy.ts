// Changing one Job's own copy from its lanes (30 Oct 2026): `+` adds a step to this Job, the lanes mark it at once as made by the person, and Keep for every Job proposes the workflow
// the Job holds for the others. Played in a wide window, where Lanes open in the Job's spine as a section; a narrower one opens them as a dialog, so no target here names which. A mock, no Fleet behind it.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const group = (name: string) => role("group", name, { exact: true });
const views = role("toolbar", "Views", { exact: true });
const JOB_LANES = group("Workflow");
const FORM = role("form", "Add a step to this Job", { exact: true });
const CHANGES = region("Changed on this Job", { exact: true });

export const workspacesLanesCopy = walk("workspaces-lanes", [
  toSessions,
  { press: deckRow("Add gift cards to the cart"), say: "A Job on the feature workflow" },
  { press: inside(views, button("Workflow", { exact: true })), say: "The Job's Workflow opens" },
  { press: button("Add to this Job", { exact: true }), say: "+ adds to this Job's own copy: a step, a lifecycle or a rule" },
  { press: role("menuitem", "Step", { exact: true }), say: "A step" },
  { type: "Dispatch", into: inside(FORM, role("textbox", "Label", { exact: true })), say: "Its label" },
  { press: inside(FORM, button("Add", { exact: true })), say: "It is added after the step the Job is at, to this Job only" },
  { look: inside(JOB_LANES, button("Step: Dispatch", { exact: true })), say: "The lanes draw it at once, marked as changed on this Job" },
  { look: inside(CHANGES, text("Added after Plan by you")), say: "Each change is listed under the lanes, with who made it" },
  { press: inside(CHANGES, button("Keep for every Job", { exact: true })), say: "Keep for every Job proposes the workflow this Job holds for the others" },
  { look: inside(CHANGES, role("img", "Proposed for every Job", { exact: true })), say: "It waits on Approve on the Workflows page" },
], { width: 2000, height: 1200 });

export { workspacesLanesCopy as "workspaces-lanes-copy" };
