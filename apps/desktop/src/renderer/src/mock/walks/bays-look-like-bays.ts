// The Cockpit's Bays panel, each bay drawn as a berth under a heading for its repository. The walk stands on the panel and reads each
// state in turn, then lights a bay and its holder's row from either end. Over `bays-look-like-bays`.

import { button, inside, region, role, walk } from "../walk";

const bays = region("Bays");
const list = region("Jobs", { exact: true });
const bay = (name: RegExp) => inside(bays, role("button", name));

const baysLookLikeBays = walk("bays-look-like-bays", [
  { look: bays, say: "The Bays panel: the bays grouped under a heading for each repository, each a berth with its state glyph and its number in a door down the left, and who holds it to the right" },
  { look: inside(bays, role("heading", "armada", { exact: true })), say: "A repository is a heading, as in the Jobs list. The bay carries no repository of its own" },
  { look: bay(/^slot-4, armada, Held/), say: "A bay a Job holds and is working in: the state glyph pulses" },
  { hover: inside(bay(/^slot-4, armada, Held/), role("img", "Held", { exact: true })), say: "Held" },
  { look: bay(/^slot-2, armada, Held/), say: "A bay a Job holds, idle: the holder, its branch and how long it has held it" },
  { look: bay(/^slot-7, armada, Free/), say: "A free bay that is warm, its frame open and empty" },
  { hover: inside(bay(/^slot-7, armada, Free/), role("img", "Warm", { exact: true })), say: "Warm" },
  { look: bay(/^slot-5, armada, Free/), say: "A free bay that is cold" },
  { hover: inside(bay(/^slot-5, armada, Free/), role("img", "Cold", { exact: true })), say: "Cold" },
  { look: bay(/^slot-6, armada, Stranded/), say: "A stranded bay, hatched, with what it left behind" },
  { hover: inside(bay(/^slot-6, armada, Stranded/), role("img", "2 files uncommitted", { exact: true })), say: "Files uncommitted" },
  { hover: inside(bay(/^slot-6, armada, Stranded/), role("img", "1 commit not pushed", { exact: true })), say: "Commits not pushed" },
  { look: bay(/^slot-2, pocket, Closed/), say: "A closed bay, shuttered and faint" },
  { hover: bay(/^slot-3, armada, Held/), say: "The pointer on a held bay lights the row of the Job that holds it in the list, with the quiet tint a row takes under the pointer, not the selection's" },
  { hover: inside(list, button("Pin the store clock", { exact: true })), say: "And on a row, the bays it holds are lit" },
  { press: bay(/^slot-3, armada, Held/), say: "A press selects a bay, and its pane opens" },
  { look: bay(/^slot-3, armada, Held/), say: "The selected bay is framed in the accent" },
]);

export { baysLookLikeBays as "bays-look-like-bays" };
