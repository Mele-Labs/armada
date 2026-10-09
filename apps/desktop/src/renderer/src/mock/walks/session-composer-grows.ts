// The message box grows with what is typed, up to half the height of its panel, and then scrolls.
// Told on a Session's composer and on Helm's.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";

const lines = (count: number) => Array.from({ length: count }, (_, index) => `Line ${index + 1} of the message`).join("\n");

function steps(): Step[] {
  const { message } = kit(false);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("CI timeout hunt")), say: "A Session, with its message box at rest" },
    { type: lines(5), into: message, say: "Five lines, and the box has grown with them" },
    { type: lines(40), into: message, say: "Forty lines: the box stops at half the panel and scrolls inside" },
    { type: "", into: message, say: "Cleared, and the box is back to its resting height" },
  ];
}

const helm: Step[] = [
  { press: button("Helm"), say: "Helm" },
  { type: lines(5), into: role("textbox", "Ask Helm"), say: "Five lines, and Helm's box has grown" },
  { type: lines(40), into: role("textbox", "Ask Helm"), say: "Forty lines: it stops at half the dock and scrolls" },
];

const keeping: Step[] = [
  { press: inside(region("Sessions"), button("CI timeout hunt")), say: "A Session" },
  { type: "Half a thought before I leave", into: kit(false).message, say: "Something typed and not sent" },
  { press: button("Cockpit", { exact: true }), say: "Off to the Cockpit" },
  { press: button("Sessions", { exact: true }), say: "And back to the Sessions" },
  { press: inside(region("Sessions"), button("CI timeout hunt")), say: "The same Session" },
  { look: kit(false).message, say: "The message box has what was typed, with the caret at its end" },
];

const session = walk("session-thread-polish", [toSessions, ...steps()]);
const talking = walk("session-thread-polish", [toSessions, ...helm]);

const kept = walk("session-thread-polish", [toSessions, ...keeping]);

export { kept as "session-composer-keeps", session as "session-composer-grows", talking as "session-composer-grows-helm" };
