// One walk window for every walk, in at most two rows. The first row is a dropdown on the walk on screen, a dropdown on its takes,
// Reload and Capture as icons, Approve and Hide Job. ⌥⌘↓ opens the walks, ⌥⌘→ and ⌥⌘← step through them, ⌥⌘T opens the takes
// and ⌘↵ approves. Over the `walk-queue` scenario.

import { button, inside, role, walk } from "../walk";

const frame = role("dialog", /^Bridge's window on/);
const walks = inside(frame, button(/^Walks, /));
const takes = inside(frame, button(/^Takes, /));

export const walkQueue = walk("walk-queue", [
  { key: "Alt+Meta+ArrowDown", on: walks, say: "⌥⌘↓ opens the walks" },
  { press: role("menuitem", /Store tests/), say: "Choose a walk" },
  { key: "Alt+Meta+ArrowRight", on: walks, say: "⌥⌘→ is the next walk" },
  { key: "Alt+Meta+t", on: takes, say: "⌥⌘T opens the takes" },
  { press: role("menuitem", /Task marks/), say: "Pick a take" },
  { key: "Meta+Enter", on: walks, say: "⌘↵ approves" },
]);
