// A Job's Thread head when it is full. The head gives the name and state a row with the way out, and puts the repository, the acts and the figures on a row under it, which
// overflows into the ellipsis menu: the repository goes to its icon first, then the acts one at a time. Auto land is on and a subagent is running, so both glyphs are blue and
// the running one pulses. `?take=overflow` plays the same walk with one row and the name held at 18 characters. Over the `thread-head-room` scenario. Mock only.

import { button, deckRow, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = (name: string) =>
  region(`Pane: ${name}`, {
    exact: true,
  });
const thread = inside(
  pane("Thread"),
  role("tab", "Thread", {
    exact: true,
  }),
);
const acts = inside(pane("Thread"), role("toolbar", "Job acts"));
const more = inside(
  pane("Thread"),
  button("More", {
    exact: true,
  }),
);

const threadHeadRoom = walk(
  "thread-head-room",
  [
    toSessions,
    {
      press: deckRow("Retry the slot lease when two Jobs ask for the same bay at once"),
      say: "A Job opens on Default",
    },
    {
      look: acts,
      say: "The Thread head at its normal width",
    },
    {
      look: inside(acts, button("Auto land")),
      say: "Auto land is on, in blue",
    },
    {
      look: inside(acts, button("Subagents")),
      say: "A subagent is running, in blue and pulsing",
    },
    {
      key: "Meta+d",
      on: thread,
      say: "⌘D splits the pane, and the Thread is half as wide",
    },
    {
      look: more,
      say: "The head at half the width",
    },
    {
      press: more,
      say: "The ellipsis menu opens",
    },
    {
      look: role("menuitem", "Requests"),
      say: "The acts that left the head are rows here, each with its glyph",
    },
  ],
  {
    width: 1800,
    height: 900,
  },
);

export { threadHeadRoom };
