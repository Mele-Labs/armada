// A Job that waits on another and is not started. Approve in its head starts it when its trigger fires; unapproved it opens as a draft.
// Below, the workflow, model and effort it starts on. Over `waiting-job-start`.

import { button, deckRow, inside, place, region, role, walk } from "../walk";

const page = region("Job: Document Approval", { exact: true });

const waitingJobStart = walk("waiting-job-start", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: deckRow(/Document Approval/), say: "Document Approval" },
  { hover: inside(page, button("Approve", { exact: true })), say: "Approve" },
  { press: inside(page, button("Approve", { exact: true })), say: "Approved" },
  { look: inside(page, button("Approved", { exact: true })), say: "Starts when Sketch Approval Needs You lands" },
  { look: inside(page, role("combobox", "Workflow")), say: "Workflow" },
  { look: inside(page, role("combobox", "Model")), say: "Model" },
  { look: inside(page, role("combobox", "Effort")), say: "Effort" },
]);

export { waitingJobStart };
