// Hovering a wave on an Epic's strip reads the whole approach its plan
// recorded, of which the tab names the first sentence (#1692, 23.14).

import { tab, walk } from "../walk";

export const waveApproachOnHover = walk("epic/wave-off-the-wire", [
  { look: tab("Wave 2 · The seam first"), say: "Each wave is named by the first sentence of its plan" },
  { hover: tab("Wave 2 · The seam first"), say: "Hovering it reads the whole approach" },
  { hover: tab("Wave 1 · The seam as one Job"), say: "and the first wave keeps its own" },
]);
