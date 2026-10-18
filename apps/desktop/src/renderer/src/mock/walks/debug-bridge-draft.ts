// Debug Bridge opens a Session holding its note and the picture as a draft, and no turn runs until
// Enter sends it. A running turn has a Stop beside the working mark: it ends the turn, the Session
// rests, and its box takes the next message. Over the `debug-bridge-draft` scenario.

import { button, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const thread = region("Thread");
  const message = role("textbox", "Message");
  return [
    { key: "Meta+Shift+d", on: place("Cockpit"), say: "⇧⌘D, from wherever focus is" },
    { look: text("Work out what is going on in Bridge"), say: "A new Job, its note waiting in the box" },
    { look: text(/annotation-.*\.png/), say: "The screenshot attached to it" },
    { look: thread, say: "No turn is running: the thread is empty" },
    { key: "Enter", on: message, say: "Enter sends it, and the agent starts" },
    { hover: inside(thread, button("Stop")), say: "Stop sits beside the working mark" },
    { press: inside(thread, button("Stop")), say: "The turn ends" },
    { look: message, say: "The Job rests, and its box is ready" },
    toSessions,
    { press: inside(region("Jobs", { exact: true }), button("Backfill dry run")), say: "A Job mid-turn, in its tools" },
    { hover: inside(thread, button("Stop")), say: "Stop beside the working mark on the tool row" },
    { press: inside(thread, button("Stop")), say: "The same" },
  ];
}

const planned = walk("debug-bridge-draft", steps());
export { planned as "debug-bridge-draft" };
