// The Checks page on a repository whose run sheet has not been read: the merge line's Checks draw,
// and the checkout's own are the only ones missing. A mock, no Fleet behind it.

import { button, inside, role, text, viaPalette, walk } from "../walk";

const PANEL = role("dialog", "Check");

export const checksWithoutRunSheet = walk("checks-without-run-sheet", [
  ...viaPalette("Checks", "Checks, from the rail"),
  { look: button("desktop_test", { exact: true }), say: "The merge line's Check, with no run sheet read" },
  { press: button("desktop_test", { exact: true }), say: "Open the merge line's Check" },
  { look: inside(PANEL, text(/connection\.test\.ts/)), say: "Its log, followed as it prints" },
]);
