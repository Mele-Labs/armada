// What Needs you holds. A reply asks nothing of him, so it is filed under Idle with an unread mark and
// raises no call; a done stays under Needs you; and an item an agent stated can be removed, with a reason
// or without, the agent told in its thread. Over `needs-me-replied-and-delete`.

import { button, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const sessions = region("Work", { exact: true });
const row = (title: string) => inside(sessions, button(title, { exact: true }));
const reason = role("textbox", /^Reason for removing/);

const needsMeRepliedAndDelete = walk("needs-me-replied-and-delete", [
  { look: row("Cache key"), say: "Cache key has two items its agent stated. Its row is yellow and pulses" },
  { look: row("Changelog script"), say: "A done is a call too, so its row pulses" },
  { hover: inside(row("Pin the store clock"), role("img", "Unread reply")), say: "A reply asks nothing of him: a faint mark and no pulse" },
  { press: row("Cache key"), say: "Cache key opened" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, on the composer" },
  { press: inside(needs, button(/^Open Confirm the old fixture/)), say: "A press on its row opens the first item" },
  { hover: inside(needs, button(/^Remove/)), say: "Remove, on an item the agent stated" },
  { press: inside(needs, button(/^Remove/)), say: "Remove opens a reason box" },
  { type: "The cache tests still read it.\n", into: reason, say: "A reason, then Enter removes the item" },
  { look: text(/Owner removed "Confirm the old fixture can go": The cache tests still read it/), say: "The agent was told, with his reason" },
  { look: row("Pin the store clock"), say: "The reply keeps its mark until he opens the Session" },
  { look: inside(row("Changelog script"), role("img", "Done")), say: "Done" },
  { press: inside(needs, button(/^Open Pick a name/)), say: "The panel is back on its list: the second item opens the same way" },
  { press: inside(needs, button(/^Remove/)), say: "Remove, on the item opened" },
  { type: "\n", into: reason, say: "Enter with no reason removes it" },
  { look: text(/Owner removed "Pick a name/), say: "The agent was told without one" },
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "The reply opened" },
  toSessions,
  { look: inside(row("Pin the store clock"), role("img", "Idle")), say: "Opened once, it reads as idle, with no mark" },
]);

export { needsMeRepliedAndDelete as "needs-me-replied-and-delete" };
