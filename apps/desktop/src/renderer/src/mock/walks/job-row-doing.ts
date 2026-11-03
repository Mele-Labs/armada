// Each row's mark says what the Job is doing, and a Job that stopped twice calls for him with no item. Over `job-row-doing`.

import { deckRow, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", {
  exact: true,
});

export const jobRowDoing = walk("job-row-doing", [
  {
    hover: inside(jobs, role("img", "Running: Run the store tests")),
    say: "The turn ended with a background command still running. This row used to read as idle",
  },
  {
    hover: inside(jobs, role("img", "Subagent working")),
    say: "The turn ended with a subagent at work",
  },
  {
    hover: inside(jobs, role("img", "Working")),
    say: "A turn is running",
  },
  {
    hover: inside(jobs, role("img", "Stopped")),
    say: "Fleet told it once, and it stopped again with nothing running and nothing asked",
  },
  {
    press: deckRow("Theme token audit"),
    say: "Its thread: what it said, Fleet's one nudge, and what it said after",
  },
  {
    look: role("textbox", "Message"),
    say: "A message from you starts it again, and its next stop gets its own nudge",
  },
]);
