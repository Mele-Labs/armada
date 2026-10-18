// A reply that asks and replies that only tell. The asking one is a call under Needs you with the line that asks; the two
// that tell raise no call and sit below Needs you, above the idle Session, in the Work list and the Timeline's lanes. Over `reply-asks-or-tells`.
// Takes of how an unread reply is marked: `?take=sorted` (the mark as it was) and `?take=marked` (accent glyph, label weight title), on both surfaces.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const timeline = region("Timeline");
const list = region("Work", { exact: true });
const incoming = region("Incoming calls");
const needs = region("Needs you");

const replyAsksOrTells = walk("reply-asks-or-tells", [
  { look: inside(incoming, button(/Retry cap/)), say: "Retry cap ended its turn on a question, so it is a call. Incoming calls holds it alone" },
  { look: deckRow("Retry cap"), say: "Needs you holds it, first in the Work list" },
  { look: deckRow("Pin the store clock"), say: "Pin the store clock and Trim the log lines only told him something. They raise no call and sit below Needs you" },
  { look: deckRow("Migration notes"), say: "Migration notes is idle and comes after them, though it spoke last" },
  { press: inside(region(/^Flight control/), button(/^Timeline/)), say: "The Timeline, from the flight controls" },
  { look: inside(timeline, role("list", "Waiting")), say: "The lanes keep the same order: the two replies that told, then Migration notes" },
  { hover: inside(timeline, role("img", "Unread reply")), say: "Unread reply, on a lane that asks nothing" },
  { key: "j", on: list, say: "j walks the list in the order it is drawn" },
  { press: inside(incoming, button(/Retry cap/)), say: "Retry cap's call opens the Session" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, at the foot of the thread" },
  { look: inside(needs, text(/Should the retry cap be five or ten/)), say: "The line that asks" },
]);

export { replyAsksOrTells as "reply-asks-or-tells" };
