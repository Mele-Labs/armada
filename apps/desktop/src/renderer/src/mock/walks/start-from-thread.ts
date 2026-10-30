// A Job's agent started two Jobs, each held on its start card. Each one's Started row in the parent's thread has Start at its end, the same
// press as the Needs you row's: it starts the Job without opening it. The second Job's request is still being read, so Start shows Starting
// until the reading ends; then the Job has started and its Started row has no Start. Over `child-job-start`.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const thread = region("Thread");

const startFromThread = walk("child-job-start", [
  toSessions,
  { press: inside(jobs, button("Debug the retry cap")), say: "A Job whose agent started a Job for each fix" },
  { look: inside(thread, button("Start Settings loader and the cap", { exact: true })), say: "Each Started row has Start at its end" },
  { press: inside(thread, button("Start Retry cap per repository", { exact: true })), say: "Start on the Job still being read, from its row" },
  { look: inside(thread, role("img", "Starting", { exact: true })), say: "Starting until the reading ends" },
  { look: inside(thread, role("button", "Retry cap per repository, Started", { exact: true })), say: "The Job has started. Its Started row has no Start" },
]);

export { startFromThread as "start-from-thread" };
