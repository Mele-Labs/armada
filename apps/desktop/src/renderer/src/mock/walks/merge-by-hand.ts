// A form edit's pull request the forge would not auto-merge, so a person merges it (#2396). Three places say so, each with the
// Merge act Sessions has: a call, a person's glyph on the pull request in the merge queue with Merge on its
// pane, and the Manifest form's line under Save, whose number opens the pull request panel Sessions uses. Over `merge-by-hand`.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const queue = region("Merge queue");
const pane = region("Pull request #2405");
const form = role("tabpanel");
const panel = dialog("Pull request #2405");

export const mergeByHand = walk("merge-by-hand", [
  { look: inside(queue, button(/^#2405/)), say: "#2405 is on the merge queue" },
  { hover: inside(queue, role("img", "Needs merging by hand")), say: "A person's glyph after its number: Needs merging by hand" },
  { press: inside(queue, button(/^#2405/)), say: "A press on the chip opens #2405 on the stage" },
  { look: inside(pane, button("Merge")), say: "The pane has Merge" },
  { press: button("Open the Manifest", { exact: true }), say: "The Manifest" },
  { press: button("Choose a repository"), say: "Choose a repository" },
  { press: role("menuitem", "armada"), say: "armada" },
  { press: tab("Edit"), say: "Edit, the form" },
  { look: inside(form, button("#2405", { exact: true })), say: "Under Save: #2405" },
  { look: inside(form, button("Merge")), say: "And Merge" },
  { press: inside(form, button("#2405", { exact: true })), say: "#2405 opens the pull request panel Sessions uses" },
  { look: inside(panel, button("Merge")), say: "Its Merge is the same act" },
  { press: inside(panel, button("Close")), say: "Closed" },
  { press: inside(form, button("Merge")), say: "Merge under Save" },
  { look: inside(form, button("Save")), say: "#2405 merged: the line under Save is gone" },
]);
