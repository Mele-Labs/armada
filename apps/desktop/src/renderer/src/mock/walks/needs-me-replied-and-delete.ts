// What Needs you holds. A reply asks nothing of him, so it is filed under Idle with an unread mark and
// raises no call; a done stays under Needs you; and an item an agent stated can be removed, with a reason
// or without, the agent told in its thread. Over `needs-me-replied-and-delete`.

import { button, inside, region, role, tab, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const timeline = region("Timeline");
const sessions = region("Sessions");
const row = (title: string) => inside(sessions, role("listitem", title));
const call = (title: RegExp) => region(title);
const reason = role("textbox", /^Reason for removing/);
const toCockpit = { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back to the Cockpit" } as const;

const needsMeRepliedAndDelete = walk("needs-me-replied-and-delete", [
  { press: tab("Active"), say: "Active, the filter that reads everything live" },
  { look: inside(timeline, role("list", "Needs you")), say: "Needs you holds the done and the Session with two items. The reply is not in it" },
  { hover: inside(timeline, role("img", "Unread reply")), say: "The reply is under Waiting, wearing the unread mark" },
  { press: tab("Your move"), say: "Your move: what is asked of him. The reply raised no call" },
  { look: call(/^Session: Cache key/), say: "Cache key has two items its agent stated. This is the first" },
  { hover: inside(call(/^Session: Cache key/), button(/^Remove/)), say: "Remove, on a card for an item the agent stated" },
  { press: inside(call(/^Session: Cache key/), button(/^Remove/)), say: "Remove opens a reason box" },
  { type: "The cache tests still read it.\n", into: reason, say: "A reason, then Enter removes the item" },
  { look: call(/^Session: Cache key/), say: "The item went, and the second is in front" },
  { look: inside(region("Waiting"), text("Changelog script")), say: "A done is still a call, behind it" },
  toSessions,
  { look: row("Pin the store clock"), say: "The reply is under Idle" },
  { hover: inside(row("Pin the store clock"), role("img", "Unread reply")), say: "Unread reply: the mark stays until he opens the Session" },
  { look: inside(row("Changelog script"), role("img", "Done")), say: "Done is under Needs you" },
  { press: inside(sessions, button(/Cache key/)), say: "Cache key opened" },
  { look: text(/Owner removed/), say: "The agent was told, with his reason" },
  { press: inside(region("Needs you"), button(/^Needs you/)), say: "The second item, in the band at the foot of the thread" },
  { press: inside(region("Needs you"), button(/^Remove/)), say: "Remove on the row in the thread" },
  { type: "\n", into: reason, say: "Enter with no reason removes it" },
  { look: text(/Owner removed "Pick a name/), say: "The agent was told without one" },
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "The reply opened" },
  toSessions,
  { look: inside(row("Pin the store clock"), role("img", "Idle")), say: "Opened once, it reads as idle, with no mark" },
]);

export { needsMeRepliedAndDelete as "needs-me-replied-and-delete" };
