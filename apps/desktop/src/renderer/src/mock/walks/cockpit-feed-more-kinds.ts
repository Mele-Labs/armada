// The Cockpit's timeline with the quiet kinds: a Session's reads, edits and commands are one dim
// row each, shorter than a turn, an ask or a pull request, a glyph in the spine where they have a dot, and
// open to the files or commands they count. Over `feed-more-kinds`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const timeline = region("Timeline");
const subjects = role("navigation", "Subjects");

const cockpitFeedMoreKinds = walk("feed-more-kinds", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { press: tab("Feed"), say: "Feed" },
  { look: timeline, say: "The last fifteen minutes, newest first. Reads, edits and commands are muted and short; asks, pull requests and turns keep their weight" },
  { look: text("Waiting: delete the unused tokens?"), say: "A Job waiting on him is a diamond in the ask colour, as an ask is" },
  { look: text("Resolved: delete the unused tokens?"), say: "And its resolution, in the passed colour" },
  { hover: inside(timeline, button(/Read 22 files/)), say: "A coalesced row opens to what it counts" },
  { press: inside(timeline, button(/Read 22 files/)), say: "The files, newest first" },
  { look: text("packages/tokens/src/spacing.css"), say: "One to a line, under the row, the spine running through" },
  { press: inside(timeline, button(/Read 22 files/)), say: "Pressing again closes it" },
  { press: inside(timeline, button(/Ran 5 commands/)), say: "Commands open the same way" },
  { look: text("cargo test -p store flaky -- --test-threads=1"), say: "Each command" },
  { look: text("Check the headings against the style guide"), say: "A subagent that has not finished turns" },
  { look: text("Subagent finished"), say: "A finished one is a still glyph in the passed colour" },
  { look: text("Started by Migration notes"), say: "A Job another Job started carries who started it" },
  { look: text("store: 1 failed"), say: "A pull request's Check going red keeps its weight among the reads and edits" },
  { look: text("Checks passed"), say: "And going green" },
  { press: tab("Lanes"), say: "Lanes" },
  { hover: inside(timeline, button(/Edited 3 files/)), say: "On a Job's track a read, an edit or a command is a small grey mark; its card names the row and its newest file or command" },
  { press: tab("Split"), say: "Split" },
  { press: inside(subjects, button(/Job: Migration notes/)), say: "A Job picked" },
  { look: timeline, say: "Its own rows, the quiet ones in the same weight" },
]);

export { cockpitFeedMoreKinds as "cockpit-feed-more-kinds" };
