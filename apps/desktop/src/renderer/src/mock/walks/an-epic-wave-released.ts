// An Epic's plan at its gate, with the wave it proposed as real Jobs, and one
// press of Approve the plan releasing all of them (spike 022, slice 6: #1694,
// served since 23.11).

import { button, card, tab, walk } from "../walk";

export const anEpicWaveReleased = walk("epic/plan-review", [
  { press: tab("Plan"), say: "The plan waits on you, with the wave it proposed" },
  { look: tab("Wave 1"), say: "The first wave is history: its Jobs ran" },
  { look: card("Refuse an unknown code at the seam"), say: "Each proposed Job is a real Job, waiting for approval" },
  { look: card("Drop the second error shape"), say: "and none of them is approved alone" },
  { press: button("Approve the plan"), say: "One press releases the whole wave" },
  { look: card("Refuse an unknown code at the seam"), say: "Every Job of it is queued" },
  { look: card("Drop the second error shape"), say: "including the last one" },
]);
