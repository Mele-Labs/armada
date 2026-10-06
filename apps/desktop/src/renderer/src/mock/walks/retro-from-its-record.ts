// A Job's own retro, from its detail: the Record's head carries Retro, and it
// opens the sheet the Retros page opens, each item drawn the same way and
// answered the same way. The walk updates Kit from a Kit item and rejects a Manifest
// item there, then reads Accepted on Retros. Job 3; `retro.test.tsx` holds
// the claims.

import { button, dialog, inside, role, tab, text, walk } from "../walk";

const RETRO = dialog("Retro");
const item = (words: string) => role("listitem", words);
const GREP = item("A Drone had to wait for grep to be allowed");
const DOCS = item("A docs edit ran every Rust test");

export const retroFromItsRecord = walk("retro/job-3", [
  { press: tab("Record"), say: "Job 3's Record" },
  { press: button("Retro", { exact: true }), say: "Retro, in the Record's head" },
  { look: inside(RETRO, text(/Browser tests timed out under the gate/)), say: "The same sheet Retros opens" },
  { hover: inside(GREP, role("img", "Lands in Kit")), say: "Where each fix lands, beside whose way" },
  { press: inside(GREP, button("Evidence")), say: "Each item's evidence, behind a control" },
  { look: inside(RETRO, text(/Allow grep on \.armada\/checks/)), say: "The question it cites" },
  { hover: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit on a Kit item with a command" },
  { press: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit adds the command and saves it" },
  { press: inside(DOCS, button("Reject change")), say: "Reject change discards it" },
  { press: inside(RETRO, button("Close")), say: "Back to the Record" },
  { press: button("Retros", { exact: true }), say: "Every Job's items, on Retros" },
  { press: tab("Accepted"), say: "Accepted: the saved Kit items" },
  { look: GREP, say: "The Kit item, saved" },
]);
