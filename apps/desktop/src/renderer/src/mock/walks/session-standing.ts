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
const tray = role("group", "Calls waiting");
const strip = (title: RegExp) => inside(tray, button(title));
const call = (title: RegExp) => region(title);
const row = (title: string) => inside(sessions, role("listitem", title));
const toCockpit = { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back to the Cockpit" } as const;

const sessionStanding = walk("session-standing", [
  { look: tray, say: "Three Sessions need him, and two of them never asked for anything" },
  { press: strip(/^Session: Cache key/), say: "The question the agent asked opens first. The reply and the done wait with it, raised from what happened and not from a tool call" },
  { look: call(/^Session: Cache key/), say: "The question" },
  { key: "Escape", on: cockpit, say: "Put away" },
  { look: tab("Active"), say: "Active, the filter that reads everything live, on the timeline" },
  { look: inside(timeline, role("list", "Needs you")), say: "The Session that replied and the one that is done stand in Needs you, beside the one that asked. Neither is moving, and neither is stuck, though each ended on a live turn" },
  { hover: inside(timeline, role("img", "Replied")), say: "A reply: its turn ended on a message to him" },
  { hover: inside(timeline, role("img", "Done")), say: "Done: the agent said nothing is left" },
  { look: inside(timeline, role("list", "Stuck")), say: "Stuck is the one Session that went quiet mid-turn, and nothing else" },
  { look: inside(timeline, role("list", "Moving")), say: "Moving is the one at work" },
  { press: button("Grid"), say: "The tiles again" },
  { look: tray, say: "The calls wait in the tray" },
  toSessions,
  { look: inside(row("Pin the store clock"), role("img", "Replied")), say: "The same Sessions, with the same marks. A reply is the Reply glyph, and on hover its name" },
  { look: inside(row("Changelog script"), role("img", "Done")), say: "Done is the power glyph, where the row used to read Idle" },
  { look: inside(row("Theme token audit"), role("img", "Stuck")), say: "And the quiet one reads Stuck here as it does on the timeline" },
  toCockpit,
  { press: strip(/^Session: Cache key/), say: "Open the question" },
  { key: "o", on: cockpit, say: "Open the Session that asked, to see what it asked" },
  toCockpit,
  { press: strip(/^Session: Pin the store clock/), say: "His question is gone from the calls: he was in the Session after it began waiting. The reply opens: the agent's first line with a box to answer in" },
  { type: "Good. Merge it.\n", into: inside(call(/^Session: Pin the store clock/), role("textbox", "Message")), say: "Answered in words" },
  { press: strip(/^Session: Changelog script/), say: "The reply left the moment it was sent. The done opens" },
  { look: button(/Keep going/), say: "Done offers two acts: end the Session, or keep going" },
  { press: button(/End session/), say: "End session" },
  { look: tab("Active"), say: "Nothing is left to answer" },
]);

export { sessionStanding as "session-standing" };
