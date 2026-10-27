// Settings, Bridge, Spacing: Compact, Default and Roomy, as Gmail offers them (26 Oct 2026). Default is Bridge as it was; Compact is a
// quarter tighter and Roomy a quarter looser, rows, panels, controls and gaps alike, and type keeps its size. Over the `flight-control` scenario.

import { place, region, role, toSystemPage, walk } from "../walk";

const spacing = role("combobox", "Spacing");

const spacingSetting = walk("flight-control", [
  { look: region("Bays"), say: "Default" },
  ...toSystemPage("Spacing", "Spacing, under Bridge"),
  { type: "compact", into: spacing, say: "Compact" },
  { press: place("Cockpit"), say: "Compact" },
  { look: region("Bays"), say: "Rows, bays and gaps closer together. Type and keys keep their size" },
  ...toSystemPage("Spacing", "Spacing"),
  { type: "roomy", into: spacing, say: "Roomy" },
  { press: place("Cockpit"), say: "Roomy" },
  { look: region("Bays"), say: "More room around everything" },
  ...toSystemPage("Spacing", "Spacing"),
  { type: "default", into: spacing, say: "Back to Default" },
]);

export { spacingSetting as "spacing-setting" };
