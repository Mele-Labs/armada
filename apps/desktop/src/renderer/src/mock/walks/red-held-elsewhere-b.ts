// Second take of `red-held-elsewhere`: the claim in the band's head, where Dispatch and Send back
// were, and no Fixing row until a Job here has it. Over `red-held-elsewhere-b`.

import { button, dialog, inside, region, role, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const ASK = dialog("Take over from nick-mbp");

const FIXING = "Fix components_test on main";

const heldElsewhereB = walk("red-held-elsewhere-b", [
  { press: role("button", "Merge line", { exact: true }), say: "The merge line" },
  { look: RED, say: "Main is red" },
  { look: inside(RED, role("group", /^Job 7be40d15, work-mac-mini/)), say: "Another machine's Job holds it, in the band's head" },
  { hover: inside(RED, role("img", "work-mac-mini")), say: "The machine, by its mark" },
  { hover: inside(RED, role("img", /^Claimed \d+m ago$/)), say: "How long ago it was claimed" },
  { look: inside(RED, button("Take over")), say: "Take over sits beside it" },
  { later: inside(RED, button("Take over")), say: "Hours pass" },
  { hover: inside(RED, role("img", /^Claimed 3h ago, stale$/)), say: "A claim hours old reads stale by its clock and its age" },
  { press: inside(RED, button("Take over")), say: "Take over" },
  { look: inside(ASK, role("group", /^Session 9c74d82e, nick-mbp/)), say: "Asked once, with the claim in it" },
  { press: inside(ASK, button("Take over", { exact: true })), say: "Replace it" },
  { later: inside(RED, button("Take over")), say: "The claim is ours" },
  { look: inside(RED, button(FIXING)), say: "Fixing here" },
]);

export { heldElsewhereB as "red-held-elsewhere-b" };
