// The End dialog belongs to the Job it was asked on (30 Oct 2026): End job lists what is in flight, and going to another Job shuts that list rather than carrying it over.
// A mock, no Fleet behind it.

import { button, deckRow, dialog, inside, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

export const endDialogLeavesWithTheJob = walk("workspaces-lanes", [
  toSessions,
  { press: deckRow("Fix cart rounding"), say: "A Job with its deploy still rolling out" },
  { press: inside(role("region", /^Job: /), button("More")), say: "End job is behind the three dots" },
  { press: role("menuitem", "End job"), say: "End lists what is in flight first" },
  { look: dialog("In flight"), say: "The list is up over the Job" },
  { press: deckRow("Add gift cards to the cart"), say: "Another Job" },
  { look: role("region", /^Job: /), say: "No list over it: the dialog was for the other Job and went with it" },
]);

export { endDialogLeavesWithTheJob as "end-dialog-leaves-with-the-job" };
