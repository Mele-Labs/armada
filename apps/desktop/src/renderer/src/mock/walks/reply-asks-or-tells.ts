// A reply that asks and replies that only tell. The asking one is a call with the line that asks; the two that tell are
// unread and lead the idle Session in the Cockpit's list and the Timeline's lanes. Over `reply-asks-or-tells`.
// Takes of how an unread reply is marked: `?take=sorted` (the mark as it was) and `?take=marked` (accent glyph, label weight title).

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const timeline = region("Timeline");
const list = region("Work", { exact: true });
const incoming = region("Incoming calls");
const needs = region("Needs you");

const replyAsksOrTells = walk("reply-asks-or-tells", [
  { press: inside(region(/^Flight control/), button(/^Timeline/)), say: "The Timeline, from the flight controls" },
  { look: inside(timeline, role("list", "Needs you")), say: "Retry cap ended its turn on a question, so it is under Needs you" },
  { look: inside(timeline, role("list", "Waiting")), say: "Pin the store clock and Trim the log lines only told him something. They lead the lanes that are waiting, above Migration notes, which spoke last" },
  { hover: inside(timeline, role("img", "Unread reply")), say: "Unread reply, on a lane that asks nothing" },
  { look: deckRow("Retry cap"), say: "In the Work list Retry cap leads, and the two unread replies follow it" },
  { look: deckRow("Migration notes"), say: "Migration notes is idle and sits below them" },
  { key: "j", on: list, say: "j walks the list in the order it is drawn" },
  { press: inside(incoming, button(/Retry cap/)), say: "Retry cap's call opens the Session" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, at the foot of the thread" },
  { look: inside(needs, text(/Should the retry cap be five or ten/)), say: "The line that asks" },
]);

export { replyAsksOrTells as "reply-asks-or-tells" };
