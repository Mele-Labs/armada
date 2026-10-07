// Triggers on a workflow (7 Oct 2026): a Command or a skill that fires at a moment in a Job. Set
// from the workflow editor on a step, or from the list every workflow takes, and kept in the
// repository or on this machine alone. The mock Fleet answers as Fleet does: the machine's copy
// replaces the repository's, a repository's save waits for main, and a machine Trigger on a
// Command the repository lacks is skipped.

import { button, card, dialog, inside, role, text, walk } from "../walk";

const DELIVER = dialog("handoff");
const TRIGGER = dialog("gate");
const NEW = dialog("New trigger");

export const aTriggerOnAWorkflow = walk("workflows", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { look: role("region", "Triggers on every workflow"), say: "Triggers on every workflow, each marked by where it is set" },
  { hover: role("img", "This machine"), say: "This machine: only the owner's" },
  { hover: role("img", "This repository"), say: "This repository: shared, in .armada/" },
  { hover: role("img", /not a Command this repository declares/), say: "Skipped here: the repository does not declare that Command" },
  { look: role("button", /gate, PR opened, this repository, replaced/), say: "The repository's copy, struck through where the machine's replaces it" },
  { press: role("button", /gate, PR opened, this machine$/), say: "The machine's copy opens in the panel" },
  { look: TRIGGER, say: "When, what it applies to, where it is set, what it runs" },
  { look: inside(TRIGGER, role("group", "If it fails")), say: "If it fails: Block the Job and Self repair, each its own switch" },
  { press: inside(TRIGGER, button("Close")), say: "Close" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { look: inside(card("Implement"), text("Triggers on pass")), say: "A step carries the Triggers that fire at it, on a line each" },
  { press: card("Handoff"), say: "A step opens in the panel" },
  { look: inside(DELIVER, role("switch", "Draft PR")), say: "The delivering step opens its PR as a draft" },
  { look: inside(DELIVER, role("list", "Triggers on handoff")), say: "Triggers on this step, each marked by where it is set" },
  { press: inside(DELIVER, button("Add trigger on PR opened")), say: "Add a Trigger on PR opened" },
  { look: NEW, say: "Its fields, set in the repository until moved" },
  { type: "fmt", into: inside(NEW, role("combobox", "Command")), say: "A Command the repository declares" },
  { press: inside(NEW, button("Save", { exact: true })), say: "Save" },
  { hover: inside(dialog("fmt"), role("img", "Runs once it is on main")), say: "A repository's Trigger runs once it is on main" },
]);
