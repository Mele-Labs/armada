// One walk window for every walk. The bar's row names the walks waiting, oldest shown first; a walk that arrives joins the row and
// never takes the page; Approve moves to the next and the last one closes the window. The Job beside the page is the Cockpit's own
// view of the Job on screen. ⌥⌘→ and ⌥⌘← step through the walks, ⌘↵ approves. Over the `walk-queue` scenario.

import { button, inside, region, role, walk } from "../walk";

const waiting = role("toolbar", "Walks waiting");
const fold = region("Job: Fold Jobs into Sessions");

export const walkQueue = walk("walk-queue", [
  { later: waiting, say: "One walk is on screen and three wait in the row: two Jobs and a pull request, oldest first. The Job beside the page is the one on screen" },
  { key: "Meta+Enter", on: waiting, say: "Another walk from Pin the store clock joined the end of the row and the page did not change. ⌘↵ approves this walk" },
  { key: "Alt+Meta+ArrowRight", on: waiting, say: "The next walk is on screen and the Job beside it changed with it. ⌥⌘→ goes to the next walk and ⌥⌘← to the one before" },
  { press: inside(waiting, button("Fold Jobs into Sessions")), say: "A pull request, held by a Job. Pressing an entry jumps to it" },
  { later: fold, say: "The plan card waits for approval, and Stop is on the thread while the turn works" },
  { hover: inside(waiting, role("img", "Asking")), say: "Store tests asked something. Its entry is marked and the page did not change" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves this walk" },
  { key: "Meta+Enter", on: waiting, say: "The walk that asked is on screen. ⌘↵ approves it" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves it, and the pull request is the last walk" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ on the last walk closes the window" },
]);
