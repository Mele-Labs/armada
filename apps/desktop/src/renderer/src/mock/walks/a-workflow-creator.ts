// The Workflow creator, a rail row of its own: every workflow Fleet resolved
// with the place it came from, one open to edit with a scope, a refusal and a
// saved state, and the draft handed to Helm. A mock, no Fleet behind it.

import { button, inside, role, text, walk } from "../walk";

const BAY = (name: string) => role("listitem", name);
const RELEASE_NOTES = BAY("release_notes, kit");
const FRAME = role("region", "Workflow definition");

export const aWorkflowCreator = walk("every-state", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { look: role("list", "Workflow files"), say: "Every file, by the place it came from" },
  { look: BAY("bug, repository"), say: "Repository bug" },
  { look: BAY("bug, carried, overridden"), say: "Carried bug, overridden" },
  { look: BAY("hotfix, kit, left out"), say: "Left out, with its reason" },
  { press: inside(RELEASE_NOTES, button("release_notes")), say: "A Kit file opens as a draft" },
  { look: role("radiogroup", "Scope"), say: "Scope: this Manifest or Kit" },
  { press: role("radio", "Loop"), say: "Structure changed to a loop" },
  { press: button("Save", { exact: true }), say: "Save" },
  { look: text("Not saved"), say: "Refused, with the rule it broke" },
  { look: inside(FRAME, text(/^Refused$/)), say: "The band says where it stands" },
  { press: role("radio", "Sequence"), say: "Back to a sequence" },
  { press: role("radio", "This Manifest"), say: "Written to this repository instead of Kit" },
  { press: button("Save", { exact: true }), say: "Save" },
  { look: inside(FRAME, text(/^Saved$/)), say: "Saved" },
  { look: BAY("release_notes, repository"), say: "The repository file takes the id" },
  { look: BAY("release_notes, kit, overridden"), say: "The Kit file is overridden" },
  { press: button("Discuss with Helm"), say: "The draft goes to Helm" },
  { look: inside(FRAME, role("img", "Handed to Helm")), say: "Handed over" },
  { press: button("New workflow"), say: "A new definition, Kit by default" },
  { press: button("Save", { exact: true }), say: "Save with nothing filled in" },
  { look: text("Not saved"), say: "Refused: the ids are empty" },
  { type: "hotfix", into: role("textbox", "Workflow id"), say: "The workflow's id" },
  { type: "patch", into: role("textbox", "Step id"), say: "The step's id" },
  { press: button("Save", { exact: true }), say: "Save" },
  { look: inside(FRAME, text(/^Saved$/)), say: "Saved to Kit" },
]);
