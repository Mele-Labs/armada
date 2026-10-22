// A Job's Field as the owner used it: the whole repository in the side panel, the small card inside it, the card floated over the app
// while the pointer is in the panel, a pinned drone that stops writing handing the card on, the hover card entered from a drone, and a
// walk left open while the drone writes. Over the `job-field-in-use` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const field = region("Field", { exact: true });
const strip = inside(field, role("group", "Drones"));
// Found anywhere: floated, the card is drawn on the body, outside the panel.
const lens = (file: string) => role("group", `Lens: ${file}`, { exact: true });
const main = inside(strip, button(/^Fix the Field on real Jobs: Writing FieldPanel\.tsx$/));

export const jobFieldInUse = walk("job-field-in-use", [
  { press: deckRow("Fix the Field on real Jobs"), say: "The Job is open. Its Field shows the whole repository, with this Job's drones on it" },
  { look: inside(field, lens("areas.rs")), say: "A subagent is writing areas.rs. Its card is small and stands over the map, inside the panel" },
  { hover: strip, say: "The pointer comes into the panel" },
  { hover: lens("areas.rs"), say: "The card floats over the app at full size, past the panel's edge. The pointer can rest on it" },
  { press: main, say: "A press on the Job's drone in the strip pins the card to it" },
  { look: lens("FieldPanel.tsx"), say: "The card is on FieldPanel.tsx" },
  { later: lens("FieldPanel.tsx"), say: "Now the Job's drone stops writing and reads a doc" },
  { look: lens("areas.rs"), say: "The card goes to the subagent, which still writes" },
  { hover: inside(field, role("option", /^Rust, Job, /)), say: "The pointer on the subagent's drone opens its card" },
  { hover: inside(role("tooltip"), text("crates/store")), say: "The pointer crosses to the card, and the card stays open" },
  { later: field, say: "Now the subagent finishes, and the Job opens a walk while its drone writes layout.ts" },
  { look: lens("layout.ts"), say: "The walk is open and the drone still writes: it stays at layout.ts, and the card shows its change" },
]);
