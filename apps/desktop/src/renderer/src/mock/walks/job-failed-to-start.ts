// A Job whose agent never started, even on Fleet's second try: its row wears a state mark with the cause in its tooltip and pulses as a row that
// needs you does, its thread ends on Fleet's line with the cause and the agent's own error lines, and Start again puts the Job to work.
// Over `job-failed-to-start`.

import { button, deckRow, inside, role, text, walk } from "../walk";

const failedRow = deckRow("Rename the slot pool");
const line = role("region", "Failed to start");

const jobFailedToStart = walk("job-failed-to-start", [
  { look: failedRow, say: "This Job did not start. Its row is tinted and pulses like any row that needs you" },
  { hover: inside(failedRow, role("img", /^Failed to start/)), say: "The tooltip names the cause" },
  { press: failedRow, say: "Open the Job" },
  { look: text("Did not start: the agent did not come up within 10 seconds"), say: "Fleet tried twice. The line names the cause. Start again and Report sit at its end" },
  { press: inside(line, button("Error lines")), say: "The agent's own last error lines" },
  { press: inside(line, button("Start again")), say: "Start again" },
  { look: inside(failedRow, role("img", "Working")), say: "The Job is working" },
]);

export { jobFailedToStart as "job-failed-to-start" };
