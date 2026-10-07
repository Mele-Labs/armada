// Merge at the review gate says what the forge's checks are and what the press did. Passed: Merge,
// and the Job completes merged. Running: Enable auto-merge, and the Job stays with Auto-merge on.
// Failed: Merge is off with the names of the checks.

import { button, dialog, inside, text, walk } from "../walk";

export const theMergeAtTheGatePassed = walk("gate/checks-passed", [
  { look: text("Checks passed"), say: "The pull request's checks, on its card" },
  { look: button("Merge pull request"), say: "Passed: Merge" },
  { press: button("Merge pull request"), say: "Press it" },
  { press: inside(dialog("Merge this job's pull request?"), button("Merge pull request")), say: "Confirm" },
  { look: text("Merged"), say: "The pull request reads Merged and the Job is done" },
]);

export const theMergeAtTheGateRunning = walk("gate/checks-running", [
  { look: text("Checks running"), say: "Still running, on the card" },
  { look: button("Enable auto-merge"), say: "Running: Enable auto-merge, where Merge was" },
  { press: button("Enable auto-merge"), say: "Press it" },
  { press: inside(dialog("Merge when the checks pass?"), button("Enable auto-merge")), say: "Confirm" },
  { look: text("Auto-merge on"), say: "Auto-merge on, drawn on the card and on the control. The Job stays at the gate" },
]);

export const theMergeAtTheGateFailed = walk("gate/checks-failed", [
  { look: text("Checks failed: ci / test, ci / lint"), say: "The failing checks, named on the card" },
  { look: text("ci / test, ci / lint failed"), say: "Merge is off, with the same names under it" },
]);
