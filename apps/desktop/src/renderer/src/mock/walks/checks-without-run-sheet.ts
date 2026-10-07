// The Checks page on a repository whose run sheet has not been read: a Job's gate Check, a Drone's
// runs and the merge line's Checks all draw, and the checkout's own are the only ones missing.
// A mock, no Fleet behind it.

import { button, inside, role, text, walk } from "../walk";

const PANEL = role("dialog", "Check");

export const checksWithoutRunSheet = walk("checks-without-run-sheet", [
  { press: button("Checks", { exact: true }), say: "Checks, from the rail" },
  { look: button("components_test", { exact: true }), say: "A gate's Check on a Job, with no run sheet read" },
  { look: button("scripts_test", { exact: true }), say: "A Drone's run beside it" },
  { look: button("desktop_test", { exact: true }), say: "The merge line's Check" },
  { press: button("components_test", { exact: true }), say: "Open the gate's Check" },
  { look: inside(PANEL, text(/182 passed/)), say: "Its log, read from the Job" },
]);
