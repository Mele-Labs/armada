// A pull request whose check is red does not always weigh the same. The required check red is the
// failure; a shard red while the run goes on is lighter; a shard red after the required checks passed
// is lighter still. None of them lights the Session's row, and none is under Needs you. Over the
// `session-checks-weights` scenario.

import { inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });

export const sessionChecksWeights = walk("session-checks-weights", [
  toSessions,
  { look: inside(sessions, role("img", /^Pull request #2301, Checks failed/)), say: "The required check red: the pull request has failed" },
  { look: inside(sessions, role("img", /^Pull request #2302, Checks running; failing/)), say: "A shard red while the run goes on: still running, and the shard is named" },
  { look: inside(sessions, role("img", /^Pull request #2303, Checks passed; not required/)), say: "A shard red after the required checks passed: passed, and the shard is named" },
  { look: inside(sessions, role("img", /^Pull request #2304, Checks passed$/)), say: "All green, for comparison" },
]);
