// A step added at the gate (7 Oct 2026), a mock: the approval canvas offers the
// + on the connector in every gap, since nothing has started: ahead of the first
// step, between steps, and around the pull request and the merge in the Delivery
// lane. The line runs through it. An added step sits on the spine between its
// neighbours, drawn as it is on the running Job, and is there on the Job's
// Workflow tab once it is dispatched.

import { button, region, role, tab, walk } from "../walk";

export const addAStepAtDispatch = walk("proto/feature-at-approval", [
  { look: region("What you are approving"), say: "A Job at its gate" },
  { hover: button("Add a step before Plan the change"), say: "A + ahead of the first step: nothing has started" },
  { press: button("Add a step after Plan the change"), say: "A + on the connector between steps" },
  { look: role("menu"), say: "A Script, a Skill or a Drone step, over the canvas" },
  { press: role("menuitem", "Skill"), say: "A Skill" },
  { press: role("button", /Close/), say: "Close" },
  { look: role("button", /^qa-notes, Added to this Job only/), say: "On the spine, and the canvas has panned to it" },
  { hover: button("Add a step after Pull request"), say: "A + in the Delivery lane, after the PR opens" },
  { press: button("Add a step after Pull request"), say: "Add a step there" },
  { press: role("menuitem", "Script"), say: "A Script: deploy to QA" },
  { press: role("button", /Close/), say: "Close" },
  { look: role("button", /^deploy_qa, Added to this Job only/), say: "After the PR opens, in the Delivery lane" },
  { press: tab("Workflow"), say: "The Job's Workflow tab" },
  { look: role("button", /^qa-notes, Added to this Job only/), say: "The first step, carried through" },
  { look: role("button", /^deploy_qa, Added to this Job only/), say: "And the one after the PR opens, after the step that delivers" },
]);
