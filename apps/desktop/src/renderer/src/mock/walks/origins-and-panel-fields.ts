// The owner's five glyph picks, wired (3 Oct 2026), and the approval panel
// editing the request, the tiers and the Drone cap (2 Oct 2026): a request
// filling in with its workflow cell blinking, a Job at its gate whose criteria
// carry each origin's mark and an issue that moved, and that Job edited in the
// panel, approved, and read back.

import { button, inside, region, role, tab, text, walk } from "../walk";

const REQUEST = "Make the stat say what is running";
const AT_THE_GATE = "Retire guide 8 and add guide validation rule";
const APPROVING = region("What you are approving");
const DONE_WHEN = inside(APPROVING, region("Done when"));
const ASKED = "Retire guide 8, add the validation rule, and name the guide it refuses";

/** The Board's row for a Job, by the words on it. */
const boardRow = (words: string) => role("option", words);

export const originsAndPanelFields = walk("origins-and-panel-fields", [
  { press: button("Dispatch", { exact: true }), say: "Dispatch the request" },
  {
    hover: inside(boardRow(REQUEST), role("img", "Workflow, still being settled")),
    say: "Its row arrives with the workflow cell blinking: the proposer has not settled it",
  },
  { look: inside(boardRow(REQUEST), text("feature")), say: "The workflow lands, and the caret gives way to it" },
  { press: boardRow(AT_THE_GATE), say: "Open a Job waiting at its gate" },
  { hover: inside(DONE_WHEN, role("img", "From an issue")), say: "A criterion read from an issue: the ticket, then the issue" },
  {
    hover: inside(DONE_WHEN, role("img", "The issue has been edited since Fleet read it")),
    say: "The issue was edited after Fleet read it, dated on hover",
  },
  { hover: inside(DONE_WHEN, role("img", "From your request")), say: "A criterion from the request typed at dispatch" },
  { press: inside(APPROVING, button("Add a criterion")), say: "Add a line at the gate" },
  {
    type: "The refusal names the guide it refused",
    into: inside(APPROVING, role("textbox", "Criterion 4", { exact: true })),
    say: "A person wrote it",
  },
  { hover: inside(DONE_WHEN, role("img", "Written or reworded by a person")), say: "So it carries the person's mark" },
  { type: ASKED, into: inside(APPROVING, role("textbox", "What was asked")), say: "The request, rewritten in the panel" },
  { type: "opus", into: inside(APPROVING, role("combobox", "Difficult")), say: "Difficult work runs on opus" },
  { type: "3", into: inside(APPROVING, role("spinbutton", "Drones at once")), say: "Three Drones at once" },
  { press: button("Approve dispatch"), say: "Approve sends what moved" },
  { look: text(ASKED), say: "The Job's brief is the request as rewritten" },
  { press: tab("Settings"), say: "What froze at the press" },
  { look: inside(region("Frozen at approval"), text("opus")), say: "Difficult froze to opus" },
  { look: inside(region("Frozen at approval"), text(/^3$/)), say: "and three Drones at once" },
]);
