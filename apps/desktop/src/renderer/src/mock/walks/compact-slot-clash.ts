// The Bays key over every bay a Session holds, and a write clash that raises no call in the thread: Fleet tells both agents, so there is
// nothing in it for him. Over the `compact-slot-clash` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const compactSlotClash = walk("compact-slot-clash", [
  { press: deckRow("Retry the slot lease"), say: "This Session holds two bays and clashes with two other Sessions. The thread's last card has the question, and no call for the clashes" },
  { look: role("region", "Write clash"), say: "The clashes are one notice under the header: Write clash, 2 other Sessions" },
  { press: role("button", /^Bays$/), say: "The Bays key is lit by the clash. It opens every bay the Session holds" },
  { look: inside(region("Bays"), role("region", "slot-4")), say: "Each bay has its branch, its state and how far it is behind main. Show in Bays opens it there, and Back returns here" },
  { look: inside(region("Bays"), role("table", "Write clashes")), say: "The clashes are a table under the bays: the other Session, linked, and the files" },
  { press: inside(region("Bays"), button("5 more")), say: "A long list shows its first three files. The chip shows them all" },
  { press: inside(region("Bays"), button("Open crates/fleet/src/lease.rs")), say: "A file opens its diff panel: this Session's change to it, then the other Session's" },
  { look: inside(role("dialog", "crates/fleet/src/lease.rs"), role("img", "Collides with Rotate the audit log")), say: "The hunk both change is marked in each, in the clash hue" },
  { key: "Escape", on: role("dialog", "crates/fleet/src/lease.rs"), say: "Esc closes the panel. Bays is as it was" },
  { hover: inside(region("Bays"), button("Open Rotate the audit log")), say: "Hovering a Session's name lights its row in Work" },
  { press: inside(region("Bays"), button("Open Rotate the audit log")), say: "The Session's name opens it. Back returns here" },
]);

export { compactSlotClash };
