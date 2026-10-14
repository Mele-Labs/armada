// The other machine's Sessions on the Sessions page, beside this machine's: a laptop on each row that is
// the other's, the state and what it waits on as it published them, and nothing to press. One changes
// state where it stands, then the whole machine goes quiet. Over `other-machine-sessions`.

import { inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Sessions");
const row = (title: string) => inside(sessions, role("listitem", title));
const MACHINE = "MacBook Air";

const otherMachineSessions = walk("other-machine-sessions", [
  toSessions,
  { look: row("Pin the store clock"), say: "This machine's: no mark" },
  { look: row("Cap the retry backoff"), say: "The other machine's: a laptop on the row, and what it waits on" },
  { hover: inside(row("Cap the retry backoff"), role("img", MACHINE)), say: "The laptop names the machine" },
  { look: row("Pin the CI toolchain"), say: "Running there" },
  { look: row("Trim the changelog"), say: "Done there" },
  { look: inside(row("Port the sync adapter"), text("harbor-sync")), say: "A repository this machine does not have" },
  { later: inside(row("Pin the CI toolchain"), role("img", "Working")), say: "It changes state on the other machine" },
  { look: inside(row("Pin the CI toolchain"), text("Allow the toolchain download?")), say: "Running became waiting, in place" },
  { later: inside(row("Pin the CI toolchain"), role("img", MACHINE)), say: "The other machine stops being heard from" },
  { look: inside(row("Pin the CI toolchain"), role("img", /^Last heard/)), say: "Its rows go quiet" },
  { hover: inside(row("Cap the retry backoff"), role("img", /^Last heard/)), say: "How long since it was last heard" },
]);

export { otherMachineSessions as "other-machine-sessions" };
