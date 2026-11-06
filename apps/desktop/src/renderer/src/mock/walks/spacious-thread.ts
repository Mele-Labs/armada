// A Job's thread drawn with more room, to set beside today's: one centred column, the body in a larger sans with looser
// lines, labels in sentence case, messages parted by space and not boxes. It is the app's look; `?thread=today` plays
// the same steps over the look before it. Then a Job thinking, one making calls, one whose agent reasoned and one with nothing
// left to do. Over
// the `spacious-thread` scenario.

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
  {
    press: deckRow("Landing page copy"),
    say: "A Job whose agent is thinking",
  },
  {
    look: inside(thread, role("img", "Working")),
    say: "Thinking, open on what it has reasoned so far",
  },
  {
    press: deckRow("Notes for 0.4"),
    say: "A Job whose agent is making calls",
  },
  {
    look: inside(thread, text(/Searching Breaking/)),
    say: "Each call on a line of its own, the running one marked",
  },
  {
    press: deckRow("Retry budget"),
    say: "A Job whose agent reasoned before it answered",
  },
  {
    press: inside(thread, text("Thought for 14s")),
    say: "How long it reasoned. Pressed, it opens to what it reasoned",
  },
  {
    press: deckRow("Quiet the slot logs"),
    say: "A Job with nothing left to do",
  },
  {
    look: region("Nothing left to do"),
    say: "Its offer to end, with what it merged",
  },
]);

export { spaciousThread as "spacious-thread" };
