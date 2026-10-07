// Hooks on a workflow (7 Oct 2026), a mock: a script or a skill that fires at a
// moment in a Job. Set from the workflow editor on a step, or from the list of
// hooks every workflow takes, and kept in one of three places: Armada's default,
// the repository, or this machine alone. No Fleet behind it.

import { button, card, dialog, inside, role, text, walk } from "../walk";

const DELIVER = dialog("handoff");
const HOOK = dialog("deploy_qa");

export const aHookOnAWorkflow = walk("workflows", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { look: role("region", "Hooks on every workflow"), say: "Hooks on every workflow, each marked by where it is set" },
  { hover: role("img", "This machine"), say: "This machine: only the owner's" },
  { hover: role("img", "This repository"), say: "This repository: shared, in .armada/" },
  { look: role("button", /deploy_qa, PR opened, this repository, overridden/), say: "The repository's hook, replaced by the machine's" },
  { press: role("button", /deploy_qa, PR opened, this machine/), say: "The machine's hook opens in the panel" },
  { look: HOOK, say: "When, what it applies to, where it is set, what it runs" },
  { look: inside(HOOK, role("group", "If it fails")), say: "If it fails: Block the Job and Self repair, each its own switch" },
  { press: inside(HOOK, button("Close")), say: "Close" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { look: inside(card("Implement"), text("Hooks on pass")), say: "A step carries the hooks that fire at it, on a line each" },
  { press: card("Handoff"), say: "A step opens in the panel" },
  { look: inside(DELIVER, role("switch", "Draft PR")), say: "The delivering step opens its PR as a draft" },
  { look: inside(DELIVER, role("list", "Hooks on handoff")), say: "Hooks on this step, each marked by where it is set" },
  { press: inside(DELIVER, button("Add hook on PR opened")), say: "Add a hook on PR opened" },
  { look: dialog("smoke"), say: "Its fields, set in the repository until moved" },
  { press: inside(dialog("smoke"), button("Close")), say: "Close" },
  { press: card("Tests"), say: "Another step" },
  { look: role("list", "Hooks on tests"), say: "A skill that runs when tests starts" },
  { press: role("button", /qa-notes, tests starts/), say: "The skill's fields" },
  { look: role("combobox", "Skill"), say: "A Skill a Drone runs, in place of a command" },
]);
