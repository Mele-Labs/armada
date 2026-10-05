// An Epic's wave read off the wire alone (#1692, 23.14): each wave named by
// what its plan split the work into, and the graph drawn in the order each Job
// waits in, off the Board's own rows.

import { card, tab, walk } from "../walk";

export const waveRoundsAndEdges = walk("epic/wave-off-the-wire", [
  { press: tab("Plan"), say: "The wave, on Plan" },
  { look: tab("Wave 2 · The seam first"), say: "Each wave is named by what its plan split the work into" },
  { look: card("Drop the second error shape"), say: "A Job sits behind the Jobs it waits on" },
  { press: tab("Wave 1 · The seam as one Job"), say: "The first wave keeps its own line" },
  { look: card("Handle every refusal at the seam"), say: "and its own order" },
]);
