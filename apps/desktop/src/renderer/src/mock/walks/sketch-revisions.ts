// A sketch the agent drew three times under one title: the pin opens on the newest, drawn as a change to the one before, and its
// revision menu reads each earlier drawing as a change to the one before it, the first plain. Over `sketch-revisions`.

import { button, deckRow, inside, role, text, walk } from "../walk";

const pinned = role("article", "Sketch to approve: Pause on the row", { exact: true });

const sketchRevisions = walk("sketch-revisions", [
  { press: deckRow("Pause a Job from its row"), say: "A Job whose sketch waits for your approval" },
  { look: inside(pinned, button(/^Revision 3/)), say: "The pin opens on revision 3, the newest" },
  { look: inside(pinned, text("Against Revision 2")), say: "Drawn as a change to revision 2" },
  { look: inside(pinned, role("group", "Added, Resume")), say: "Resume is new in it" },
  { look: inside(pinned, role("group", "Changed, Fleet")), say: "Fleet's line changed" },
  { press: inside(pinned, button(/^Revision 3/)), say: "Every revision, newest first" },
  { press: role("menuitem", "Revision 2"), say: "Revision 2" },
  { look: inside(pinned, text("Against Revision 1")), say: "Drawn as a change to revision 1" },
  { look: inside(pinned, role("group", "Added, Paused row")), say: "The paused row is what it added" },
  { look: inside(pinned, button("Approve", { exact: true })), say: "Approve still answers revision 3" },  { press: inside(pinned, button(/^Revision 2/)), say: "The menu again" },
  { press: role("menuitem", "Revision 1"), say: "Revision 1" },
  { look: inside(pinned, role("group", "Job row", { exact: true })), say: "The first drawing, plain, with nothing to compare it to" },
]);

export { sketchRevisions as "sketch-revisions" };
