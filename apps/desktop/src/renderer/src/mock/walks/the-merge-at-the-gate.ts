// Merge at the review gate says what the forge's checks are and what the press did. Passed: Merge,
// and the Job completes merged. Running: Enable auto-merge, and the Job stays with Auto-merge on.
// Failed: Merge is off with the names of the checks.

import { button, dialog, inside, role, text, walk } from "../walk";

export const theMergeAtTheGatePassed = walk("gate/checks-passed", [
  { look: role("img", "Checks passed"), say: "The pull request leads the gate, its checks a mark on the card" },
  { look: button("Merge pull request"), say: "Passed: Merge, right under it" },
  { press: button("Merge pull request"), say: "Press it" },
  { press: inside(dialog("Merge this job's pull request?"), button("Merge pull request")), say: "Confirm" },
  { look: text("Merged"), say: "The pull request reads Merged and the Job is done" },
]);

export const theMergeAtTheGateRunning = walk("gate/checks-running", [
  { look: role("img", "Checks running"), say: "Running: a turning mark on the card; hover names it" },
  { look: button("Enable auto-merge"), say: "Enable auto-merge, right under the pull request" },
  { press: button("Enable auto-merge"), say: "Press it" },
  { press: inside(dialog("Merge when the checks pass?"), button("Enable auto-merge")), say: "Confirm" },
  { look: role("img", "Auto-merge on"), say: "Auto-merge on: a mark on the card, and the control reads it too. The Job stays at the gate" },
]);

export const theMergeAtTheGateFailed = walk("gate/checks-failed", [
  { look: role("img", "Checks failed: ci / test, ci / lint"), say: "A failed mark on the card; hover names the failing checks" },
  { look: text("ci / test, ci / lint failed"), say: "Merge is off under the pull request, with the same names" },
]);
