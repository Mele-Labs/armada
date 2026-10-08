// A step added at the gate (7 Oct 2026): the approval canvas offers the + on the connector in every
// gap, since nothing has started: ahead of the first step, between steps, and before and after the
// pull request opens in the Delivery lane. The line runs through it. Steps are held until the press
// and go to Fleet as `additions`, which places them on the Job, where its Workflow tab draws them.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const NEW = dialog("Skill");
const SCRIPT = dialog("Script");

export const addAStepAtDispatch = walk("proto/feature-at-approval", [
  { look: region("What you are approving"), say: "A Job at its gate" },
  { hover: button("Add a step before Plan the change"), say: "A + ahead of the first step: nothing has started" },
  { press: button("Add a step after Plan the change"), say: "A + on the connector between steps" },
  { look: role("menu"), say: "A Script, a Skill or a Drone step, over the canvas" },
  { press: role("menuitem", "Skill"), say: "A Skill" },
  { type: "qa-notes", into: inside(NEW, role("textbox", "Skill")), say: "Its name" },
  { press: inside(NEW, button("Add", { exact: true })), say: "Add" },
  { look: role("button", /^qa-notes, Added to this Job only, pending/), say: "On the spine between its neighbours, and the canvas has panned to it" },
  { hover: button("Add a step after Pull request"), say: "A + in the Delivery lane, after the PR opens" },
  { press: button("Add a step after Pull request"), say: "Add a step there" },
  { press: role("menuitem", "Script"), say: "A Script" },
  { type: "fmt", into: inside(SCRIPT, role("combobox", "Script")), say: "A Command the repository declares" },
  { press: inside(SCRIPT, button("Add", { exact: true })), say: "Add" },
  { look: role("button", /^fmt, Added to this Job only, pending/), say: "After the PR opens, in the Delivery lane" },
  { press: button("Approve dispatch"), say: "Approve: the steps go with it" },
  { press: tab("Workflow"), say: "The Job's Workflow tab" },
  { look: role("button", /^qa-notes, Added to this Job only/), say: "The first step, carried through" },
  { look: role("button", /^fmt, Added to this Job only/), say: "And the one after the PR opens" },
]);
