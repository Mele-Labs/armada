// A command a Drone is waiting on, answered with Always allow on this machine
// (the owner, 5 Oct 2026): the choice sits beside the repository's, sends the
// rule picked, and the command is on the Kit page's allowlist as an Always
// allow's. `job-detail-plan.test.tsx` and `kit.test.tsx` hold the claims.

import { button, inside, region, role, text, walk } from "../walk";

const ALLOWLIST = region("Allowlist");

export const alwaysAllowOnThisMachine = walk("job/runningWaitingOnACommand", [
  { look: role("radio", "Always allow in this repository"), say: "The repository's own choice, as it was" },
  { look: role("radio", "Always allow on this machine"), say: "Beside it, every repository on this machine" },
  { press: role("radio", "Always allow on this machine"), say: "Pick it" },
  { look: role("radio", "pnpm add"), say: "The rule it would keep, from Fleet's candidates" },
  { press: button("Send this answer"), say: "Send" },
  { press: button("Kit", { exact: true }), say: "Kit" },
  { look: inside(ALLOWLIST, text(/^pnpm add$/)), say: "The command, in the allowlist" },
  { look: inside(ALLOWLIST, text(/^Always allow$/)), say: "Where it came from" },
  { look: inside(ALLOWLIST, button("Remove pnpm add")), say: "Remove takes it out" },
]);
