// Talk on a Mac that has not allowed the microphone: the button leaves its pressed state and a guide says what to turn on.
// Over the `talk-refused` scenario.

import { button, deckRow, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const refused = walk("talk-refused", [
  toSessions,
  {
    press: deckRow("Store clock fix"),
    say: "A Job's message box",
  },
  {
    press: button("Talk", {
      exact: true,
    }),
    say: "Talk, with the microphone not allowed",
  },
  {
    look: text(/microphone is off/i),
    say: "It stops at once and says why, with the steps",
  },
  {
    look: button("Open Microphone settings"),
    say: "This opens the Microphone pane in System Settings",
  },
]);

export { refused as "talk-refused" };
