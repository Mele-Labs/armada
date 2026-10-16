// A form edit's pull request the forge would not auto-merge, so a person merges it (#2396). Three places say so: a call in Incoming
// calls, a person's glyph on the pull request in the merge queue, and a line under Save on the Manifest's form. Over `merge-by-hand`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const incoming = region("Incoming calls");
const queue = region("Merge queue");
const call = /armada\.yml edit #2405/;
const pane = region("Pull request #2405");
const form = role("tabpanel");

export const mergeByHand = walk("merge-by-hand", [
  { look: inside(incoming, button(call)), say: "A call: armada.yml edit #2405, Needs merging by hand" },
  { look: inside(queue, button(/^#2405/)), say: "#2405 is on the merge queue" },
  { hover: inside(queue, role("img", "Needs merging by hand")), say: "A person's glyph after its number: Needs merging by hand" },
  { press: inside(incoming, button(call)), say: "A press on the call opens #2405 on the stage" },
  { look: inside(pane, text("Needs merging by hand")), say: "Merge: Needs merging by hand" },
  { press: button("Open the Manifest", { exact: true }), say: "The Manifest" },
  { press: tab("Edit"), say: "Edit, the form" },
  { look: inside(form, text("needs merging by hand")), say: "Under Save: #2405 needs merging by hand" },
  { press: inside(form, button("#2405", { exact: true })), say: "#2405 opens the pull request" },
]);
