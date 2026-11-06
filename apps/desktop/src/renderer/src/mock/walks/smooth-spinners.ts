// Small loops run smooth at the display's rate, and the Cockpit's map stays on the loop clock, 5 moves a second.
// Over the `one-session-view` scenario, which has a working Job.

import { deckRow, inside, region, text, walk } from "../walk";

const smoothSpinners = walk("one-session-view", [
  {
    press: deckRow("Rotate the audit log"),
    say: "A working Job",
  },
  {
    look: inside(region("Thread"), text("Editing audit.rs")),
    say: "Its working mark turns smoothly, at the display's rate",
  },
  {
    look: region("Field", {
      exact: true,
    }),
    say: "The drone on the map still moves 5 times a second, on the clock",
  },
  {
    look: deckRow("Fold Jobs into Sessions"),
    say: "A row that needs you pulses smoothly too. Is the spinner smooth, and the map still calm?",
  },
]);

export { smoothSpinners as "smooth-spinners" };
