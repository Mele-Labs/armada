// A step added at the gate (7 Oct 2026), a mock: the approval canvas offers the
// same + in every gap between two steps, since nothing has started. An added
// step sits on the spine between its neighbours, drawn as it is on the running
// Job, and is there on the Job's Workflow tab once it is dispatched.

import { button, region, role, tab, walk } from "../walk";

export const addAStepAtDispatch = walk("proto/feature-at-approval", [
  { look: region("What you are approving"), say: "A Job at its gate" },
  { hover: button(/Add a step after Plan the change/), say: "A + in each gap between two steps" },
  { press: button(/Add a step after Plan the change/), say: "Add a step in that gap" },
  { look: role("menu"), say: "A Script, a Skill or a Drone step, over the canvas" },
  { press: role("menuitem", "Script"), say: "A Script" },
  { press: role("button", /Close/), say: "Close" },
  { look: role("button", /^deploy_qa, Added to this Job only/), say: "On the spine between its neighbours, drawn as it is on a running Job" },
  { press: tab("Workflow"), say: "The Job's Workflow tab" },
  { look: role("button", /^deploy_qa, Added to this Job only/), say: "The same step, carried through" },
]);
