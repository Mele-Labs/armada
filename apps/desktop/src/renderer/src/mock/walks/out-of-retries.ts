// A Job stopped on a failed Check, at `awaiting_repair`: Overview's lead says
// it is out of retries and offers what Fleet's `stuck.recourse` does — Run
// Checks again and Restart step. The reframe of 29 Sep 2026 had unmounted
// both; `overview-boards.test.tsx` holds the claim.

import { button, role, text, walk } from "../walk";

const lead = role("heading", "Out of retries");

export const outOfRetries = walk("job/awaitingRepair", [
  { look: lead, say: "The step spent its retries" },
  { look: text("cargo_nextest failed"), say: "The Check that failed, and what it produced" },
  { hover: button("Run Checks again"), say: "Runs the Checks again on the work already here" },
  { hover: button("Restart step"), say: "Restarts the step on the same worktree" },
]);
