// A tag is written into the message where it was typed, as a chip in the line, and the row of what
// waits stays for files and sketches. Sent, the thread draws the same chip in the words.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, role, region, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(): Step[] {
  const { message, thread } = kit(false);
  return [
    { press: inside(region("Sessions"), button("New Session")), say: "A new Session" },
    { type: "Why did it stop, @", into: message, say: "An at sign, at the end of a question" },
    { look: role("group", "Sessions"), say: "What can be tagged opens under it" },
    { press: role("option", "Release notes script"), say: "Chosen" },
    { look: inside(message, text("Release notes script")), say: "The tag stands in the line, where the at sign was" },
    { look: role("group", "Attached"), say: "The row of what waits has nothing in it for it" },
    { press: button("Send"), say: "Sent" },
    { look: inside(thread, text("Release notes script")), say: "The thread draws it as the same chip, in the words" },
  ];
}

const inline = walk("sessions", [toSessions, ...steps()]);

export { inline as "session-inline-tags" };
