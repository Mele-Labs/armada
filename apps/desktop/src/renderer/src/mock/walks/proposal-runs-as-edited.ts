// A proposal edited under the lead and approved (spike 022, slice 4): its
// title, a criterion, a gate and where it lands are what the Job carries after
// the press, and a criterion read from an issue says so and that it moved.
// Since the approval canvas (prototype, 4 Oct 2026), each is on its own node.

import { button, card, dialog, inside, region, role, tab, text, walk } from "../walk";

const APPROVING = region("What you are approving");
const BRIEF = dialog("Brief");
const DONE_WHEN = inside(BRIEF, region("Done when"));
const LANDS_IN = inside(dialog("Land"), role("combobox", "Lands in"));

export const proposalRunsAsEdited = walk("real/proposal-from-an-issue", [
  { look: APPROVING, say: "The proposal Fleet served, under the lead, yours to change" },
  { press: card("Brief"), say: "Its words, on the Brief" },
  { look: inside(DONE_WHEN, text("armada#1162")), say: "Two criteria came from the issue" },
  { look: inside(DONE_WHEN, role("img", "The issue has been edited since Fleet read it")), say: "and the issue was edited after Fleet read it" },
  {
    type: "Retire guide 8, and refuse a guide with no drawn pieces",
    into: inside(BRIEF, role("textbox", "Title", { exact: true })),
    say: "A new title",
  },
  {
    type: "A validation rule refuses a guide with no drawn pieces, naming it",
    into: inside(BRIEF, role("textbox", "Criterion 2", { exact: true })),
    say: "A criterion reworded: still the issue's line",
  },
  { press: card("Restructure"), say: "Restructure's gate hangs beside it" },
  { press: card("Checks"), say: "its Checks" },
  { press: inside(dialog("Checks on Restructure"), role("checkbox", "You on Restructure")), say: "Restructure now stops for you" },
  { press: card("Land"), say: "Where it lands" },
  { press: LANDS_IN, say: "Lands in opens on the repository's branches" },
  { look: role("listbox", "Lands in"), say: "Read from the repository, the base first" },
  { type: "release/2026-10", into: LANDS_IN, say: "It lands in the release branch" },
  { press: button("Approve dispatch"), say: "Approve sends the proposal as you left it" },
  { look: role("heading", "Retire guide 8, and refuse a guide with no drawn pieces"), say: "The Job carries the new title" },
  { press: tab("Settings"), say: "What froze at the press" },
  { look: region("Frozen at approval"), say: "Restructure stops for you, and it lands in release/2026-10" },
  { press: tab("Plan"), say: "What it is held to" },
  { press: button("What this job is held to"), say: "Every criterion" },
  { look: text("A validation rule refuses a guide with no drawn pieces, naming it"), say: "The reworded criterion" },
]);
