// One walk window for every walk. The bar's row names the walks waiting, oldest shown first; a walk that arrives joins the row and
// never takes the page; Approve moves to the next and the last one closes the window. The Job beside the page is the Cockpit's own
// view of the Job on screen. ⌥⌘→ and ⌥⌘← step through the walks, ⌘↵ approves. Over the `walk-queue` scenario.

import { button, inside, region, role, walk } from "../walk";

const waiting = role("toolbar", "Walks waiting");
const fold = region("Job: Fold Jobs into Sessions");

export const walkQueue = walk("walk-queue", [
  { later: waiting, say: "The walks waiting, oldest first. A new one from Pin the store clock joins the end" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves the walk on screen and brings up the next" },
  { key: "Alt+Meta+ArrowRight", on: waiting, say: "⌥⌘→ next walk, ⌥⌘← the one before. The Job panel follows" },
  { press: inside(waiting, button("Fold Jobs into Sessions")), say: "Press an entry to jump to it" },
  { later: fold, say: "The Cockpit's Job view, with the plan card and Stop" },
  { hover: inside(waiting, role("img", "Asking")), say: "A Job asked about its page. Its entry is marked and the page stays" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves" },
  { key: "Meta+Enter", on: waiting, say: "⌘↵ approves" },
  { key: "Meta+Enter", on: button("Approve"), say: "The last walk, with nothing waiting behind it. ⌘↵ approves and the window closes" },
]);
