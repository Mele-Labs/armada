// Main is red and another machine holds the claim. The band names the holder by its chip and age,
// Take over replaces Dispatch and asks once, and a store that cannot be reached refuses a Dispatch.
// Over `red-held-elsewhere`; `red-held-elsewhere-b` draws the claim in the band's head.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const ASK = dialog("Take over from nick-mbp");
const DISPATCH = dialog("Dispatch a Job to fix main");

const FIXING = "Fix components_test on main";

const heldElsewhere = walk("red-held-elsewhere", [
  { press: role("button", "Merge line", { exact: true }), say: "The merge line" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("group", /^Job 7be40d15, work-mac-mini/)), say: "Another machine's Job holds it: its kind, its short id, and its machine" },
  { hover: inside(RED, role("img", "work-mac-mini")), say: "The machine, by its mark" },
  { hover: inside(RED, role("img", /^Claimed \d+m ago$/)), say: "How long ago it was claimed" },
  { look: inside(RED, button("Take over")), say: "Dispatch and Send back are not offered. Take over is" },
  { later: inside(RED, button("Take over")), say: "Hours pass" },
  { look: inside(RED, role("group", /^Session 9c74d82e, nick-mbp/)), say: "A Session holds it now" },
  { hover: inside(RED, role("img", /^Claimed 3h ago, stale$/)), say: "A claim hours old reads stale by its clock and its age" },
  { press: inside(RED, button("Take over")), say: "Take over" },
  { look: inside(ASK, role("group", /^Session 9c74d82e, nick-mbp/)), say: "Asked once, with the claim in it" },
  { look: inside(ASK, role("img", /^Claimed 3h ago, stale$/)), say: "And its age" },
  { press: inside(ASK, button("Take over", { exact: true })), say: "Replace it" },
  { later: inside(RED, button("Take over")), say: "The claim is ours" },
  { look: inside(RED, button(FIXING)), say: "Fixing here, as a Job of this machine is shown" },
  { look: inside(RED, role("img", "Working on it")), say: "Working on it" },
  { later: inside(RED, button(FIXING)), say: "The fix lands" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green" },
  { later: inside(MERGE, role("img", "Main is green")), say: "Main is red again, and nobody holds it" },
  { press: inside(RED, button("Dispatch a new Job")), say: "Dispatch a new Job" },
  { press: inside(DISPATCH, button("Dispatch", { exact: true })), say: "Dispatch it" },
  { look: inside(RED, text("Not made, store unreachable")), say: "The store could not be reached: the claim was not made" },
  { press: inside(RED, button("Try again")), say: "Try again" },
  { later: inside(RED, button("Dispatch a new Job")), say: "The claim is made" },
  { look: inside(RED, button(FIXING)), say: "Fixing here" },
]);

export { heldElsewhere as "red-held-elsewhere" };
