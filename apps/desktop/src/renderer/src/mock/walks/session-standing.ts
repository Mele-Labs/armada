// One answer to what a Session is doing, on every surface that draws it. Five Sessions: one replied to him
// and waits, one said it is done, one went quiet mid-turn, one is at work, one asked a question. The
// timeline's lanes, the Sessions page and the Cockpit's calls agree about each, and a call goes the
// moment it is answered, ended or looked at, with nobody having to tell Bridge a thing. Over
// `session-standing`.

import { button, inside, region, role, tab, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const cockpit = region("Dashboard");
const timeline = region("Timeline");
const sessions = region("Sessions");
const call = (title: RegExp) => region(title);
const row = (title: string) => inside(sessions, role("listitem", title));
const toCockpit = { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back to the Cockpit" } as const;

const sessionStanding = walk("session-standing", [
  { look: tab("Your move"), say: "Your move, the call deck over the timeline: four Sessions need him, and three of them never asked for anything" },
  { look: call(/^Session: Retry cap/), say: "The oldest is a reply from a Session in a terminal. It was raised from what happened, not from a tool call" },
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { look: inside(timeline, role("list", "Needs you")), say: "The Sessions that replied and the one that is done stand in Needs you, beside the one that asked. None is moving, and none is stuck, though each ended on a live turn" },
  { hover: inside(timeline, role("img", "Replied")), say: "A reply: its turn ended on a message to him" },
  { hover: inside(timeline, role("img", "Done")), say: "Done: the agent said nothing is left" },
  { look: inside(timeline, role("list", "Stuck")), say: "Stuck is the one Session that went quiet mid-turn, and nothing else" },
  { look: inside(timeline, role("list", "Moving")), say: "Moving is the one at work" },
  { press: tab("Your move"), say: "Back to the calls" },
  toSessions,
  { look: inside(row("Pin the store clock"), role("img", "Replied")), say: "The same Sessions, with the same marks. A reply is the Reply glyph, and on hover its name" },
  { look: inside(row("Changelog script"), role("img", "Done")), say: "Done is the power glyph, where the row used to read Idle" },
  { look: inside(row("Theme token audit"), role("img", "Stuck")), say: "And the quiet one reads Stuck here as it does on the timeline" },
  toCockpit,
  { later: cockpit, say: "He answers Retry cap in its terminal. Nothing tells Bridge" },
  { look: call(/^Session: Cache key/), say: "Its call has gone: the turn began again, and Fleet read it working. The question the agent asked is in front" },
  { key: "o", on: cockpit, say: "Open the Session that asked, to see what it asked" },
  toCockpit,
  { look: call(/^Session: Pin the store clock/), say: "His question is gone from the calls: he was in the Session after it began waiting. The reply is in front, the agent's first line with a box to answer in" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in words" },
  { look: call(/^Session: Changelog script/), say: "The reply left the moment it was sent, and the done is in front" },
  { look: button(/Keep going/), say: "Done offers two acts: end the Session, or keep going" },
  { press: button(/End session/), say: "End session" },
  { look: tab("Your move"), say: "Nothing is left to answer" },
]);

export { sessionStanding as "session-standing" };
