// Two agent sketches drawn the new way: sans words at the size of the thread's own, boxes as wide as their words,
// moved apart so none touches another, and arrows run through the gaps over the boxes. The second is the sketch from the
// owner's note of 20 Oct 2026, where boxes piled up and arrows ran under them. Over the `sketch-labels-and-box-fit` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

const FLOW = "How data moves to and from the Fleet";
const NOTICE = "Why a queued pull request still pinged";

function steps(): Step[] {
  const { message, thread, close } = kit(false);
  const flow = dialog(`Sketch ${FLOW}`);
  const notice = dialog(`Sketch ${NOTICE}`);
  return [
    { press: inside(region("Jobs", { exact: true }), button("How data moves")), say: "A Job whose agent draws" },
    { type: "Show me how Armada sends data back and forth\n", into: message, say: "Asked for a diagram" },
    { press: inside(thread, button(`Open sketch ${FLOW}`)), say: "The first sketch it drew, opened" },
    { look: inside(flow, text(/lifeboat stays up/)), say: "Words in Plex Sans at the thread's size, and Fleet's box as wide as its words before they wrap" },
    { look: inside(flow, role("img", "code: advances, retry, fails")), say: "Arrows turn square corners in the gaps and are drawn over the boxes" },
    { press: close(flow), say: "Close" },
    { type: "And the auto-merge one\n", into: message, say: "Asked for the second" },
    { press: inside(thread, button(`Open sketch ${NOTICE}`)), say: "The sketch from your note" },
    { look: inside(notice, text(/pull_notices.rs:219/)), say: "The tall box no longer covers Fix: the row under it moved down to clear it" },
    { look: inside(notice, role("img", "becomes")), say: "The arrow into Fix runs in the gap, with its word beside it" },
    { press: close(notice), say: "Close" },
  ];
}

const sketchStyle = walk("sketch-labels-and-box-fit", [toSessions, ...steps()]);

export { sketchStyle as "sketch-style" };
