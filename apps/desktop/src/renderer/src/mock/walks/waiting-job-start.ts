// A Job that waits on another and is not started. Its page says what happens when the condition is met (start at once, or open as a draft)
// and the workflow, model and effort it starts on. Over `waiting-job-start`.

import { deckRow, place, region, walk } from "../walk";

const waitingJobStart = walk("waiting-job-start", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: deckRow(/Document Approval/), say: "The waiting Job" },
  { look: region("Job: Document Approval", { exact: true }), say: "Then, Workflow, Model and Effort follow when it starts and from where" },
]);

export { waitingJobStart };
