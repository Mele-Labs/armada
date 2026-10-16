// The Cockpit's Bays panel, each bay drawn as a bay. Three looks are on trial in the mock, `?bayTake=a|b|c`; this walk stands on the
// panel and reads each state in turn. Over `bays-look-like-bays`.

import { inside, region, role, walk } from "../walk";

const bays = region("Bays");
const bay = (name: RegExp) => inside(bays, role("button", name));

const baysLookLikeBays = walk("bays-look-like-bays", [
  { look: bays, say: "The Bays panel, each bay drawn as a bay: its number painted, its state as the frame and a glyph, who holds it inside" },
  { look: bay(/^slot-4, armada, Held/), say: "A bay a Session holds and is working in: the state glyph pulses" },
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
  { press: bay(/^slot-3, armada, Held/), say: "A press selects a bay, and its pane opens" },
  { look: bay(/^slot-3, armada, Held/), say: "The selected bay is framed in the accent" },
]);

export { baysLookLikeBays as "bays-look-like-bays" };
