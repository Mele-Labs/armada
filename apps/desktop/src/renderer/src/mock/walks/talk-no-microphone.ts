// Talk on a Mac with no microphone: the button leaves its pressed state and the guide says none is available. Over the
// `talk-no-microphone` scenario.

import { button, deckRow, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const noMicrophone = walk("talk-no-microphone", [
  toSessions,
  {
    press: deckRow("Store clock fix"),
    say: "A Job's message box",
  },
  {
    press: button("Talk", {
      exact: true,
    }),
    say: "Talk, with no microphone",
  },
  {
    look: text("No microphone is available"),
    say: "It stops at once and says so",
  },
]);

export { noMicrophone as "talk-no-microphone" };
