// The same ask, and the new Job does not come up: the Job is as it was, keeps its row, and a band on its composer says so. No second call is raised. Over `job-handoff-fails`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const list = region("Jobs", { exact: true });

export const jobHandoffFails = walk("job-handoff-fails", [
  { press: deckRow("Retry the slot lease"), say: "The same ask" },
  { key: "1", on: inside(region("Needs you"), role("radio", "Carry on in a new Job", { exact: true })), say: "1 picks yes" },
  { press: inside(region("Needs you"), button("Answer")), say: "Answer sends it" },
  { look: inside(list, role("img", "Handing off")), say: "The new Job is coming up" },
  { look: inside(region(/^Job: /), button("Failed to hand off to new job")), say: "The hand off failed: a band on the composer, as Needs you is. The Job is as it was, on its row" },
]);
