// Runs on a workflow (7 Oct 2026), a mock: a script or a skill that fires at a
// moment in a Job. Set from the workflow editor on a step, or from the list of
// runs every workflow takes, and kept in one of three places: Armada's default,
// the repository, or this machine alone. No Fleet behind it.

import { button, card, dialog, inside, role, text, walk } from "../walk";

const DELIVER = dialog("deliver");
const RUN = dialog("deploy_qa");

export const aRunOnAWorkflow = walk("workflows", [
  { press: button("Workflows", { exact: true }), say: "Workflows, from the rail" },
  { look: role("region", "Runs on every workflow"), say: "Runs on every workflow, each marked by where it is set" },
  { hover: role("img", "This machine"), say: "This machine: only the owner's" },
  { hover: role("img", "This repository"), say: "This repository: shared, in .armada/" },
  { look: role("button", /deploy_qa, PR opened, this repository, overridden/), say: "The repository's run, replaced by the machine's" },
  { press: role("button", /deploy_qa, PR opened, this machine/), say: "The machine's run opens in the panel" },
  { look: RUN, say: "When, what it applies to, where it is set, what it runs" },
  { look: inside(RUN, role("group", "If it fails")), say: "If it fails: Block the Job and Self repair, each its own switch" },
  { press: inside(RUN, button("Close")), say: "Close" },
  { press: button("feature, carried"), say: "The Feature workflow" },
  { look: inside(card("Implement"), text("Runs on pass")), say: "A step carries the runs that fire at it, on a line each" },
  { press: card("Deliver"), say: "A step opens in the panel" },
  { look: inside(DELIVER, role("switch", "Draft PR")), say: "The delivering step opens its PR as a draft" },
  { look: inside(DELIVER, role("list", "Runs on deliver")), say: "Runs on this step, each marked by where it is set" },
  { press: inside(DELIVER, button("Add run on PR opened")), say: "Add a run on PR opened" },
  { look: dialog("smoke"), say: "Its fields, set in the repository until moved" },
  { press: inside(dialog("smoke"), button("Close")), say: "Close" },
  { press: card("Review"), say: "Another step" },
  { look: role("list", "Runs on review"), say: "A skill that runs when review starts" },
  { press: role("button", /qa-notes, review starts/), say: "The skill's fields" },
  { look: role("combobox", "Skill"), say: "A Skill a Drone runs, in place of a command" },
]);
