// One walk window for every walk, in at most two rows. The first walk on screen is the worst case: walks queued, three takes of its page,
// the page's own steps and a question waiting. ⌥⌘→ and ⌥⌘← step through the walks, ⌘↵ approves. Over the `walk-queue` scenario.
// Every target is one all three takes of the bar draw (`?take=tabs`, `switcher`, `panel`).

import { button, inside, region, role, walk } from "../walk";

const frame = role("dialog", /^Bridge's window on/);
const bar = inside(frame, button("Reload"));
const question = inside(frame, button("Show question"));

export const walkQueue = walk("walk-queue", [
  { later: question, say: "Queue, takes, steps and a question in two rows" },
  { key: "Alt+Meta+ArrowLeft", on: bar, say: "A walk joined. ⌥⌘← wraps to it" },
  { key: "Alt+Meta+ArrowLeft", on: bar, say: "⌥⌘← again" },
  { later: region("Job: Fold Jobs into Sessions"), say: "Its plan card and Stop" },
  { hover: inside(frame, role("img", "Asking")), say: "A queued Job asked" },
  { key: "Meta+Enter", on: bar, say: "⌘↵ approves" },
  { look: question, say: "The walk that asked, its question waiting" },
  { press: question, say: "The question over the page" },
]);
