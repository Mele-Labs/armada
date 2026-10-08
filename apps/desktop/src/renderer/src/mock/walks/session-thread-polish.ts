// A Session's thread, read on a terminal Session. Calls that ran one after another are one closed
// row that opens, a slash command is a line as typed, a compaction is a quiet row and not "You",
// and every message is left aligned on the panel with no fill. Helm's thread is told too. Told
// wide, narrow, and Helm's own.

import { NARROW, kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { thread } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("CI timeout hunt")), say: "A terminal Session" },
    { look: inside(thread, text("Conversation compacted")), say: "The compaction summary is a quiet row, and not from the person" },
    { press: inside(thread, text("Conversation compacted")), say: "It opens to its text" },
    { look: inside(thread, text("/reload-plugins")), say: "A command is one line as typed, with no tags" },
    { look: inside(thread, text("Grep, Read, Bash")), say: "Three calls in a row are one closed row naming the tools" },
    { press: inside(thread, text("Grep, Read, Bash")), say: "Pressed, it shows each call" },
    { look: inside(thread, text("Bash cat /private/tmp/store-ci/out.txt")), say: "Every call, in order" },
    { look: inside(thread, text("Wait on the write instead of sleeping, and keep the test under a second.")), say: "The person's words sit left, on the panel, with no fill" },
    { look: inside(thread, text("It sleeps 50 ms and then reads the clock. CI is slower than that, so the read lands before the write.")), say: "The agent's the same" },
  ];
}

const helm: Step[] = [
  { press: button("Helm"), say: "Helm" },
  { look: inside(role("complementary", "Helm"), text("why did 77 stop")), say: "Helm's thread: the person's words left aligned, on the panel's ground" },
];

const wide = walk("session-thread-polish", [toSessions, ...steps(false)]);
const narrow = walk("session-thread-polish", [toSessions, ...steps(true)], NARROW);
const talking = walk("helm-talking", helm);

export { wide as "session-thread-polish", narrow as "session-thread-polish-narrow", talking as "session-thread-polish-helm" };
