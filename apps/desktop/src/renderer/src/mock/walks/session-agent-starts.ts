// A Session's agent starts a Session, and forks its own conversation, when the owner asks it to. The
// new Session appears at once under Needs you, with the agent's first message in its box, unsent;
// nothing runs until he sends it. Its thread says where it came from, and the Session that started it
// lists it. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread, ledger, opened, row } = kit(narrow);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("Debug Job 14")), say: "A live Session, with a finding in it" },
    { look: inside(thread, text("The loop reads the cap once at startup")), say: "What it found" },
    { type: "Start a session to look into the settings loader ignoring a cap set for one repository\n", into: message, say: "He asks its agent to start a Session for the second finding" },
    { look: inside(thread, text("Started Settings loader and the cap.")), say: "It does" },
    ...opened([{ look: inside(role("region", "Started"), text("Started Settings loader and the cap")), say: "The Session that started it lists it" }]),
    { press: button("Sessions"), say: "Back to the list" },
    { look: inside(sessions, role("heading", "Needs you")), say: "The new Session is under Needs you" },
    { hover: inside(sessions, role("img", /^Waiting on you:/)), say: "A hand, and on hover what it waits on" },
    { press: inside(sessions, button(/Settings loader and the cap/)), say: "He opens it" },
    { look: role("button", "Started by Debug Job 14", { exact: true }), say: "Its thread opens on where it came from" },
    { look: message, say: "The agent's first message waits in its box, unsent" },
    ...opened([
      { look: inside(ledger, region("Waiting on you")), say: "And in Waiting on you" },
      { hover: inside(ledger, role("img", "Goes to the message", { exact: true })), say: "Its glyph names the act" },
      { look: inside(role("region", "Started"), text("Started by Debug Job 14")), say: "The ledger says where it came from" },
    ]),
    { press: button("Send", { exact: true }), say: "He sends it" },
    { look: inside(thread, text("A cap set for one repository never reaches it.")), say: "Now it runs" },
    { press: role("button", "Started by Debug Job 14", { exact: true }), say: "The row opens the Session that started it" },
    { type: "Fork yourself at that finding and check the loop per attempt\n", into: message, say: "He asks the agent to fork itself at its finding" },
    { look: inside(thread, text("Forked Debug Job 14, cap per attempt.")), say: "It does" },
    ...opened([{ look: inside(role("region", "Forks"), text("Forked to Debug Job 14, cap per attempt")), say: "The ledger lists the fork" }]),
    ...row("Open Forked to Debug Job 14, cap per attempt", "He opens it"),
    { press: role("button", "Forked from Debug Job 14", { exact: true }), say: "It opens onto the conversation up to that finding" },
    { look: inside(role("group", "Forked from Debug Job 14"), text("settings loader seems to ignore")), say: "Up to the message it was forked at" },
    { look: message, say: "The agent's first message waits in its box, unsent" },
    ...opened([{ look: inside(ledger, region("Waiting on you")), say: "Waiting on you, as the other was" }]),
  ];
}

const wide = walk("session-agent-starts", [toSessions, ...steps(false)]);
const narrow = walk("session-agent-starts", [toSessions, ...steps(true)], NARROW);

export { wide as "session-agent-starts", narrow as "session-agent-starts-narrow" };
