// A Job's thread drawn with more room, to set beside today's: one centred column, the body in a larger sans with looser
// lines, labels in sentence case, messages parted by space and not boxes. `?thread=spacious` draws the take; without it,
// the same steps show today's thread. Over the `spacious-thread` scenario.

import { deckRow, inside, region, role, text, walk } from "../walk";

const thread = region("Thread");

const spaciousThread = walk("spacious-thread", [
  {
    press: deckRow("Demo marketing video"),
    say: "A Job's thread",
  },
  {
    look: inside(thread, region("Your message")),
    say: "Your words",
  },
  {
    look: inside(thread, text(/The Board opens on three Jobs/)),
    say: "The agent's words",
  },
  {
    look: inside(thread, text("Read multiple files")),
    say: "Calls the agent made, folded to one line",
  },
  {
    look: inside(thread, text(/vitest run/)),
    say: "A run and its last lines",
  },
  {
    look: inside(
      thread,
      role("article", "Document: Video script, Revision 1", {
        exact: true,
      }),
    ),
    say: "The script, waiting on your Approve",
  },
  {
    look: region("Needs you"),
    say: "Needs you, on the composer",
  },
  {
    look: role("textbox", "Message"),
    say: "The composer",
  },
]);

export { spaciousThread as "spacious-thread" };
