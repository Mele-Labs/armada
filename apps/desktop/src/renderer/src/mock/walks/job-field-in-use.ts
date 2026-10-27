// A Job's Field as the owner used it: the whole repository in the side panel, the small card inside it, the card floated over the app
// while the pointer is in the panel, a drone that stops writing leaving the card to the one that still does, the hover card entered from a drone, and a
// walk left open while the drone writes. Over the `job-field-in-use` scenario.

import { deckRow, inside, region, role, text, walk } from "../walk";

const field = region("Field", { exact: true });
// Found anywhere: floated, the card is drawn on the body, outside the panel.
const lens = (file: string) => role("group", `Lens: ${file}`, { exact: true });

export const jobFieldInUse = walk("job-field-in-use", [
  { press: deckRow("Fix the Field on real Jobs"), say: "The Job is open. Its Field shows the whole repository, with this Job's drones on it" },
  { look: inside(field, lens("areas.rs")), say: "A subagent is writing areas.rs. Its card is small and stands over the map, inside the panel" },
  { hover: inside(field, role("listbox", "Map")), say: "The pointer comes into the panel" },
  { hover: lens("areas.rs"), say: "The card floats over the app at full size, past the panel's edge. The pointer can rest on it" },
  { later: lens("areas.rs"), say: "Now the Job's drone stops writing and reads a doc. The card stays on the subagent, which still writes" },
  { hover: inside(field, role("option", /^Keep the stamp in the store, subagent of /)), say: "The pointer on the subagent's drone opens its card" },
  { hover: inside(role("tooltip"), text("crates/store")), say: "The pointer crosses to the card, and the card stays open" },
  { later: field, say: "Now the subagent finishes, and the Job opens a walk while its drone writes layout.ts" },
  { look: lens("layout.ts"), say: "The walk is open and the drone still writes: it stays at layout.ts, and the card shows its change" },
]);
