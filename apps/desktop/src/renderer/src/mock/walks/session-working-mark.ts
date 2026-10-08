// A Session whose turn is running shows it at the end of the thread: the animated working mark,
// on the last tool group's row while the agent is in its tools.

import { button, inside, region, role, text, walk } from "../walk";

const marked = walk("session-working-mark", [
  { press: inside(region("Sessions"), button("Backfill dry run")), say: "A Session mid-turn" },
  { look: inside(region("Thread"), text("Read, Bash")), say: "The last row is a folded tool group" },
  { look: inside(region("Thread"), role("img", "Working")), say: "The working mark sits on that row" },
]);
export { marked as "session-working-mark" };
