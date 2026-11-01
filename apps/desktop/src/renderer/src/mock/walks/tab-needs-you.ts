// A Custom tab's Mod Job held on a question to him. The tab wears the needs-you mark, the Job whose panes hold the tab wears it in the Jobs list, and the Mod Job has no row of its own.
// The tab's card holds the ask at the top, with its keys, over the thread tail and the box. Allow once settles it, and the tab goes back to building. Mock only. Over the `job-layouts` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const HOLDER = "Design how a watch ends";
const pane = inside(region("Pane: Pull queue", { exact: true }), role("tab", /Pull queue/));

const steps: Step[] = [
  toSessions,
  { press: deckRow(HOLDER), say: "Open the Job whose panes hold the Pull queue tab" },
  { look: inside(region("Pane: Pull queue", { exact: true }), role("img", "Waiting on you")), say: "The tab wears the needs-you mark: its Job waits on you" },
  { look: region("Jobs", { exact: true }), say: "The Mod Job has no row in the list" },
  { look: inside(deckRow(HOLDER), role("img", /Pull queue waits on you/)), say: "The Job that holds the tab wears the mark" },
  { hover: pane, say: "Hover the tab: the card holds the ask at the top" },
  { look: inside(role("tooltip"), role("article", "Waiting on you")), say: "The permission, with its answers and keys" },
  { press: inside(role("tooltip"), button(/^Allow once/)), say: "Allow once" },
  { look: inside(region("Pane: Pull queue", { exact: true }), role("img", "List the oldest first")), say: "The tab goes back to building" },
];

const tabNeedsYou = walk("job-layouts", steps);

export { tabNeedsYou };
