// The Cockpit's timeline, second take of the quiet kinds: the reads, edits and commands a Session ran
// between two louder events are one row of glyphs, opened for the rows inside. Over `feed-more-kinds-folded`.

import { button, inside, region, role, tab, text, walk } from "../walk";

const timeline = region("Timeline");
const subjects = role("navigation", "Subjects");

const cockpitFeedFolded = walk("feed-more-kinds-folded", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { press: tab("Feed"), say: "Feed" },
  { look: timeline, say: "The last fifteen minutes, newest first. A Session's reads, edits and commands between two louder events are one row of glyphs and no words" },
  { hover: inside(timeline, button(/^Read 14 files, Edited 3 files/)), say: "Each glyph names its row on hover" },
  { press: inside(timeline, button(/^Read 14 files, Edited 3 files/)), say: "Pressed, the rows inside, in the order they happened" },
  { look: text("Edited 2 files"), say: "Each of them a row of its own" },
  { press: inside(timeline, button(/Read 14 files/)), say: "A row opens to its files as it does unfolded" },
  { look: text("crates/store/src/snapshot.rs"), say: "One to a line" },
  { press: inside(timeline, button(/^Read 14 files, Edited 3 files/)), say: "The fold closes with the rows inside" },
  { look: text("Waiting: delete the unused tokens?"), say: "Waiting and resolved are never folded" },
  { look: text("Subagent finished"), say: "Nor is a subagent, nor a pull request's Check" },
  { press: tab("Lanes"), say: "Lanes draw the same as in the other take" },
  { press: tab("Split"), say: "Split folds as the feed does" },
  { press: inside(subjects, button(/Session: Theme token audit/)), say: "A Session picked" },
  { look: timeline, say: "Its reads and commands folded on each side of the decision" },
]);

export { cockpitFeedFolded as "cockpit-feed-folded" };
