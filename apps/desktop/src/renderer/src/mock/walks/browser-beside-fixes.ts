// Three fixes to the Cockpit and the Job browser: the Jobs list's filter chip has its own lane above the first row and no longer
// covers its buttons, the Job's thread sits close to its browser with the handle between them still within reach, and Send stays on
// the thread's composer at the width it is given beside the browser. Mock only. Over the `job-browser` scenario.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const list = region("Jobs");
const tile = inside(region("Flight controls"), button("Browser", { exact: true }));
const panel = dialog("Job browser");
const pane = inside(panel, role("group", "Job browser", { exact: true }));
const floating = region(/^Job: /);

const steps: Step[] = [
  toSessions,
  { hover: list, say: "Over the Jobs list, the filter comes up in a lane of its own above the first row" },
  { look: inside(list, button("Filter", { exact: false })), say: "The first row's buttons are all clear of it" },
  { press: deckRow("Fix the checkout total"), say: "A Job fixing the checkout total" },
  { press: tile, say: "Browser" },
  { look: pane, say: "The thread and the browser sit close, one narrow gap between them" },
  { look: inside(floating, button("Send")), say: "Send is still on the thread's composer, with the menus giving up room first" },
];

const quickFixes = walk("job-browser", steps);

export { quickFixes as "browser-beside-fixes" };
