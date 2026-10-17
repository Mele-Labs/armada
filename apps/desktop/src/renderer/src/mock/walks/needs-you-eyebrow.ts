// What a Session waits on him for is a tab on the composer's top edge, a glyph an item. A press raises a sheet from that
// edge over the foot of the thread, and Esc puts it away. Over the `session-waiting-on-you` scenario.

import { button, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Work", { exact: true });
const needs = region("Needs you");
const tab = inside(needs, button(/^Needs you/));
const item = inside(needs, button("Open Approve #1843"));

const needsYouEyebrow = walk("session-waiting-on-you", [
  toSessions,
  { press: inside(sessions, button(/Pin the store clock/)), say: "A Session waiting on four things" },
  { look: tab, say: "A tab on the composer's top edge, a glyph an item" },
  { press: tab, say: "A press raises the sheet from that edge, over the thread" },
  { look: item, say: "The rows, each a press that does its act" },
  { key: "Escape", on: item, say: "Esc puts it away, and focus is back on the tab" },
  { look: tab, say: "The tab again" },
]);

export { needsYouEyebrow as "needs-you-eyebrow" };
