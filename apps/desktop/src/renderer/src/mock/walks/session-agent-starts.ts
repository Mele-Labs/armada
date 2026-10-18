// A Session's agent starts a Session, and forks its own conversation, when the owner asks it to. The
// new Session appears at once in the list, waiting on him, with the agent's first message in its box, unsent;
// nothing runs until he sends it. In the Session that started it, a line in the thread names the new
// Session and opens it with a press. Its own thread says where it came from. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread } = kit(narrow);
  const started = (title: string) => role("button", title, { exact: true });
  const sessions = region("Jobs", { exact: true });
  return [
    { press: inside(sessions, button("Debug the retry cap")), say: "A live Job, with a finding in it" },
    { look: inside(thread, text("The loop reads the cap once at startup")), say: "What it found" },
    { type: "Start a session to look into the settings loader ignoring a cap set for one repository\n", into: message, say: "He asks its agent to start a Job for the second finding" },
    { look: inside(thread, started("Settings loader and the cap")), say: "It does: a line in the thread, with the new Job's name" },
    { hover: inside(thread, started("Settings loader and the cap")), say: "It reads as a press" },
    { press: inside(thread, started("Settings loader and the cap")), say: "He opens it from the thread" },
    { look: role("button", "Started by Debug the retry cap", { exact: true }), say: "Its thread opens on where it came from" },
    { look: message, say: "The agent's first message waits in its box, unsent" },
    // A running Session's items wait in Needs you over the composer, not the ledger.
    { look: region("Needs you"), say: "And in Needs you, over the composer" },
    { press: place("Cockpit"), say: "Back to the list" },
    { hover: inside(sessions, role("img", /^Waiting on you/)), say: "The new Job has a hand on its row, and on hover what it waits on" },
    { press: inside(sessions, button(/Settings loader and the cap/)), say: "He opens it from the list as well" },
    { press: button("Send", { exact: true }), say: "He sends it" },
    { look: inside(thread, text("A cap set for one repository never reaches it.")), say: "Now it runs" },
    { press: role("button", "Started by Debug the retry cap", { exact: true }), say: "The row opens the Job that started it" },
    { type: "Fork yourself at that finding and check the loop per attempt\n", into: message, say: "He asks the agent to fork itself at its finding" },
    { look: inside(thread, started("Debug the retry cap, cap per attempt")), say: "It does: the same line, with the fork glyph" },
    { press: inside(thread, started("Debug the retry cap, cap per attempt")), say: "He opens it from the thread" },
    { press: role("button", "Forked from Debug the retry cap", { exact: true }), say: "It opens onto the conversation up to that finding" },
    { look: inside(role("group", "Forked from Debug the retry cap"), text("settings loader seems to ignore")), say: "Up to the message it was forked at" },
    { look: message, say: "The agent's first message waits in its box, unsent" },
    { look: region("Needs you"), say: "In Needs you, as the other was" },
  ];
}

const wide = walk("session-agent-starts", [toSessions, ...steps(false)]);
const narrow = walk("session-agent-starts", [toSessions, ...steps(true)], NARROW);

export { wide as "session-agent-starts", narrow as "session-agent-starts-narrow" };
