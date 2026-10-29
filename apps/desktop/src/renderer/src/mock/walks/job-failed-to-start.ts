// A Job whose agent never started: its row wears a state mark with the reason in its tooltip and pulses as a row that needs him does, and its
// thread ends on a Fleet line with the reason. A message in its box is the retry. Over `job-failed-to-start`.

import { deckRow, inside, role, text, walk } from "../walk";

const failedRow = deckRow("Rename the slot pool");

const jobFailedToStart = walk("job-failed-to-start", [
  { look: failedRow, say: "A Job that did not start: a cross mark, tinted and pulsing like any row that needs you" },
  { hover: inside(failedRow, role("img", /^Failed to start/)), say: "The tooltip names the reason" },
  { press: failedRow, say: "Open the Job" },
  { look: text("Did not start: the job's keeper did not answer"), say: "Fleet's line on the thread gives the reason. Send a message to try again" },
]);

export { jobFailedToStart as "job-failed-to-start" };
