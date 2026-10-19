// A pull request panel opened again draws what it read last time at once, and the forge's fresh read lands in the same panel
// behind it. The mock's forge takes its time the second time it is asked about #1892. Over `shared-side-panel`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const queue = region("Merge queue");
const pull = role("dialog", "Pull request #1892");

const prPanelReopensFromLastRead = walk("shared-side-panel", [
  { press: inside(queue, button(/^#1892/)), say: "A pull request in the merge queue" },
  { look: inside(pull, tab("Conversation")), say: "Read once, and drawn" },
  { key: "Escape", on: pull, say: "Esc closes the panel" },
  { press: inside(queue, button(/^#1892/)), say: "Opened again, while the forge is slow" },
  { look: inside(pull, tab("Conversation")), say: "Drawn at once from the last read, no waiting" },
  { look: inside(pull, text("Refreshed from the forge.")), say: "The refresh lands behind it, in the same panel" },
  { key: "Escape", on: pull, say: "Esc" },
]);

export { prPanelReopensFromLastRead as "pr-panel-reopens-from-last-read" };
