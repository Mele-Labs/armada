// Every sheet resizes by its inner edge, and each kind of sheet remembers its
// own width (owner, 2 Oct 2026): Pulse's log panel dragged wide leaves Plan's
// task panel at its own width. `sheet-width.test.tsx` holds the claims.

import { button, card, dialog, inside, role, tab, walk } from "../walk";

export const everySheetResizes = walk("arc/executing-sequential", [
  { press: tab("Pulse"), say: "A Job whose third group is working" },
  { press: button("Drone transcript, implement · T5"), say: "T5's transcript opens in a panel" },
  { drag: role("separator", "Resize Drone transcript"), by: { x: -240, y: 0 }, say: "Drag its edge, and the panel widens" },
  { press: inside(dialog("Drone transcript"), button("Close")), say: "Close it" },
  { press: tab("Plan"), say: "Over to the plan" },
  { press: card("Draw what is running, in four lists"), say: "T5's task panel opens at its own width" },
  { press: inside(dialog("Draw what is running"), button("Close")), say: "Close it" },
  { press: tab("Pulse"), say: "Back to Pulse" },
  { press: button("Drone transcript, implement · T5"), say: "The transcript reopens as wide as it was left" },
  { look: dialog("Drone transcript"), say: "Still wide" },
]);
