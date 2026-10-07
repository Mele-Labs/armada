// A step added to one Job from its workflow (7 Oct 2026), a mock: a `+` after
// the step the Job is on and after each step to come, offering a Script, a
// Skill or a Drone step. The added step is drawn apart from the workflow's
// own, fires on the running Job, and can be kept for every Job as a trigger.

import { button, card, dialog, inside, role, tab, text, walk } from "../walk";

const ADDED = dialog("deploy_qa");

export const addAStepToAJob = walk("proto/feature-running", [
  { press: tab("Workflow"), say: "The Job's workflow, on its canvas" },
  { hover: button("Add a step after Implement"), say: "A + after the step the Job is on, and after each step to come" },
  { press: tab("Stacked"), say: "The stacked run offers the same +" },
  { press: button("Add a step after Review the change"), say: "A step after the one that opens the PR" },
  { press: role("menuitem", "Script"), say: "A Script: a command from armada.yml, which Fleet runs" },
  { look: ADDED, say: "Its fields, for this Job only" },
  { press: inside(ADDED, role("switch", "Self repair")), say: "Self repair on, Block the Job off" },
  { look: inside(ADDED, role("group", "If it fails")), say: "The same two switches a trigger has" },
  { press: inside(ADDED, button("Close")), say: "Close" },
  { press: tab("Canvas"), say: "Back to the canvas" },
  { hover: role("button", /^deploy_qa, Added to this Job only/), say: "Drawn apart from the workflow's own steps, with the trigger mark" },
  { look: role("button", /^deploy_qa, Added to this Job only, firing/), say: "It fires on the running Job" },
  { look: role("button", /^deploy_qa, Added to this Job only, passed/), say: "And passes" },
  { press: card("deploy_qa"), say: "Open the added step again" },
  { press: inside(ADDED, button("Keep for every Job")), say: "Keep it for every Job" },
  { look: inside(ADDED, role("combobox", "Set in")), say: "The saved-trigger editor, filled in: This machine or Repository" },
  { press: inside(ADDED, button("Keep")), say: "Keep" },
  { press: card("deploy_qa"), say: "The added step says where it was kept" },
  { look: inside(ADDED, text("Kept: this machine")), say: "Kept for every Job, on this machine" },
]);
