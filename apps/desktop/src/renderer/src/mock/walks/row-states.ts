// Every Cockpit row reads as one of five by its glyph's hue: amber needs you, blue works, teal waits on something that
// is not you, grey is idle and a draft is the dormant grey. Over `row-states`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const waiting = region("Waiting", { exact: true });
const mark = (said: string) => inside(jobs, role("img", said, { exact: true }));

const rowStates = walk("row-states", [
  { look: jobs, say: "Each row's glyph wears one of five hues" },
  { hover: mark("Waiting on you"), say: "Amber, pulsing: a question waits on you" },
  { hover: mark("Stuck"), say: "Stuck is amber too" },
  { hover: mark("Checks failed on #3079"), say: "So is a red pull request with the agent idle" },
  { hover: mark("Working"), say: "Blue: working" },
  { hover: mark("#3082 in the merge queue"), say: "Teal: waiting on something else. Here, the merge queue" },
  { hover: mark("#3081 to auto-merge once Checks pass"), say: "Checks running, auto-merge on" },
  { hover: mark("Held open by Staging deploy · #409 to auto-merge once Checks pass"), say: "A deploy watch and a pull request: the tooltip names both" },
  { hover: mark("Idle"), say: "Grey: idle, nothing pending" },
  { hover: mark("Unread reply"), say: "An unread reply stays idle, with a dot" },
  { hover: mark("Not started"), say: "A draft, in the dormant grey" },
  { press: deckRow("Fix Four Filed Issues"), say: "Open a Job that waits on something else" },
  { look: waiting, say: "A teal band on the message box, a glyph for each thing it waits on" },
  { press: inside(waiting, button("Waiting")), say: "Pressed, it opens as Needs you does" },
  { look: inside(waiting, role("list", "Waiting on")), say: "A row for what it waits on" },
  { key: "Escape", on: waiting, say: "Esc folds it" },
  { press: deckRow("Raise the upload limit"), say: "A Job waiting on two things" },
  { press: inside(waiting, button("Waiting")), say: "Open its band" },
  { look: inside(waiting, role("list", "Waiting on")), say: "A row each: the Staging deploy watch, and #409 on its Checks" },
  { key: "Escape", on: waiting, say: "Esc" },
]);

export { rowStates };
