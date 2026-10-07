// Triggers on a workflow (7 Oct 2026), a mock: a script or a skill that fires at a
// moment in a Job. Set from the workflow editor on a step, or from the list of
// triggers every workflow takes, and kept in one of three places: Armada's default,
// the repository, or this machine alone. No Fleet behind it.

import { button, card, dialog, inside, role, text, walk } from "../walk";

const DELIVER = dialog("handoff");
const TRIGGER = dialog("deploy_qa");

export const aTriggerOnAWorkflow = walk("workflows", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { look: role("region", "Triggers on every workflow"), say: "Triggers on every workflow, each marked by where it is set" },
  { hover: role("img", "This machine"), say: "This machine: only the owner's" },
  { hover: role("img", "This repository"), say: "This repository: shared, in .armada/" },
  { look: role("button", /deploy_qa, PR opened, this repository, overridden/), say: "The repository's trigger, replaced by the machine's" },
  { press: role("button", /deploy_qa, PR opened, this machine/), say: "The machine's trigger opens in the panel" },
  { look: TRIGGER, say: "When, what it applies to, where it is set, what it runs" },
  { look: inside(TRIGGER, role("group", "If it fails")), say: "If it fails: Block the Job and Self repair, each its own switch" },
  { press: inside(TRIGGER, button("Close")), say: "Close" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { look: inside(card("Implement"), text("Triggers on pass")), say: "A step carries the triggers that fire at it, on a line each" },
  { press: card("Handoff"), say: "A step opens in the panel" },
  { look: inside(DELIVER, role("switch", "Draft PR")), say: "The delivering step opens its PR as a draft" },
  { look: inside(DELIVER, role("list", "Triggers on handoff")), say: "Triggers on this step, each marked by where it is set" },
  { press: inside(DELIVER, button("Add trigger on PR opened")), say: "Add a trigger on PR opened" },
  { look: dialog("smoke"), say: "Its fields, set in the repository until moved" },
  { press: inside(dialog("smoke"), button("Close")), say: "Close" },
  { press: card("Tests"), say: "Another step" },
  { look: role("list", "Triggers on tests"), say: "A skill that runs when tests starts" },
  { press: role("button", /qa-notes, tests starts/), say: "The skill's fields" },
  { look: role("combobox", "Skill"), say: "A Skill a Drone runs, in place of a command" },
]);
