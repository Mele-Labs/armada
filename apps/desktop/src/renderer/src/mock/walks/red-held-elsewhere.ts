// Main is red and another machine holds the claim. The band names the holder by its chip and age,
// Take over replaces Dispatch and asks once, then opens the Dispatch panel on a Session.
// Over `red-held-elsewhere`.

import { button, dialog, inside, region, role, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const ASK = dialog("Take over from nick-mbp");

const heldElsewhere = walk("red-held-elsewhere", [
  { press: role("button", "Merge line", { exact: true }), say: "The merge line" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("group", /^Job 7be40d15, work-mac-mini/)), say: "Another machine's Job holds it: its kind, its short id, and its machine" },
  { hover: inside(RED, role("img", "work-mac-mini")), say: "The machine, by its mark" },
  { hover: inside(RED, role("group", /^Claimed$/)), say: "How long ago it was claimed" },
  { look: inside(RED, button("Take over")), say: "Dispatch and Send back are not offered. Take over is" },
  { later: inside(RED, button("Take over")), say: "Hours pass" },
  { look: inside(RED, role("group", /^Job 9c74d82e, nick-mbp/)), say: "A Job holds it now" },
  { hover: inside(RED, role("group", /^Claimed, stale$/)), say: "A claim hours old reads stale by its clock and its age" },
  { press: inside(RED, button("Take over")), say: "Take over" },
  { look: inside(ASK, role("group", /^Job 9c74d82e, nick-mbp/)), say: "Asked once, with the claim in it" },
  { look: inside(ASK, role("group", /^Claimed, stale$/)), say: "And its age" },
  { press: inside(ASK, button("Take over", { exact: true })), say: "Replace it" },
  { look: role("textbox", "Request"), say: "The Dispatch panel opens on a Job, holding the brief" },
]);

export { heldElsewhere as "red-held-elsewhere" };
