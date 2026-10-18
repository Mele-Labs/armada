// The Bays key over every bay a Session holds, and a write clash that raises no call in the thread: Fleet tells both agents, so there is
// nothing in it for him. Over the `compact-slot-clash` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const compactSlotClash = walk("compact-slot-clash", [
  { press: deckRow("Retry the slot lease"), say: "This Job holds two bays and clashes with two other Jobs. The thread's last card has the question, and no call for the clashes" },
  { look: role("img", "Write clash with Name the store migrations, Rotate the audit log"), say: "The clash is the swords glyph on the Bays key, and its tooltip names the other Jobs. The header has no notice" },
  { press: role("button", /^Bays$/), say: "The Bays key opens every bay the Job holds" },
  { look: inside(region("Bays"), role("table", "Write clashes")), say: "The clashes come first: the other Job, linked, and the files" },
  { look: inside(region("Bays"), role("button", "Show slot-4 in Bays")), say: "Then each bay, drawn as the cockpit's Bays panel draws it, with how far it is behind main. A press shows it there, and Back returns here" },
  { press: inside(region("Bays"), button("5 more")), say: "A long list shows its first three files. The chip shows them all" },
  { press: inside(region("Bays"), button("Open crates/fleet/src/lease.rs")), say: "A file opens its diff panel: this Job's change to it, then the other Job's" },
  { look: inside(role("dialog", "crates/fleet/src/lease.rs"), role("img", "Collides with Rotate the audit log")), say: "The hunk both change is marked in each, in the clash hue" },
  { key: "Escape", on: role("dialog", "crates/fleet/src/lease.rs"), say: "Esc closes the panel. Bays is as it was" },
  { hover: inside(region("Bays"), button("Open Rotate the audit log")), say: "Hovering a Job's name lights its row in Jobs" },
  { press: inside(region("Bays"), button("Open Rotate the audit log")), say: "The Job's name opens it. Back returns here" },
]);

export { compactSlotClash };
