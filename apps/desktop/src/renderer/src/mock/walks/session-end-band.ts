// The agent's last word is a card at the end of the thread, in the done hue: what was made and the two ways on. It is part of the
// thread and never opens over it. Over the `session-all-done` scenario.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const card = region("Nothing left to do");

const sessionEndBand = walk("session-all-done", [
  toSessions,
  {
    press: inside(
      region("Jobs", {
        exact: true,
      }),
      button(/Pin the store clock/),
    ),
    say: "A Job whose agent has nothing left to do",
  },
  {
    look: card,
    say: "A card at the end of the thread",
  },
  {
    look: inside(card, button("End job")),
    say: "What was made, and the two ways on",
  },
  {
    look: inside(card, button("Keep going")),
    say: "Keep going gives the agent another turn",
  },
]);

export { sessionEndBand as "session-end-band" };
