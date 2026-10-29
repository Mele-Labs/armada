// A Job whose agent never started: its row wears a state mark with the reason in its tooltip and pulses as a row that needs him does, and its
// thread ends on a Fleet line with the reason. A message in its box is the retry. Over `job-failed-to-start`.

import { deckRow, inside, role, text, walk } from "../walk";

const failedRow = deckRow("Rename the slot pool");

const jobFailedToStart = walk("job-failed-to-start", [
  { look: failedRow, say: "The Job that did not start sits in the list with a cross mark, tinted and pulsing as any row that needs him does. The Jobs beside it started" },
  { hover: inside(failedRow, role("img", /^Failed to start/)), say: "The reason is in the mark's tooltip" },
  { press: failedRow, say: "Opening the Job" },
  { look: text("Did not start: the job's keeper did not answer"), say: "The thread ends on Fleet's line with the reason. A message in the box is how it is tried again" },
]);

export { jobFailedToStart as "job-failed-to-start" };
