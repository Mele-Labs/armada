// A reply that asks and replies that only tell. The list keeps one order and there is no panel of calls. The asking one is
// yellow and pulses while it needs him; the two that tell wear a faint mark and raise no call; the idle one has neither.
// Over `reply-asks-or-tells`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const timeline = region("Timeline");
const list = region("Work", { exact: true });
const needs = region("Needs you");

const replyAsksOrTells = walk("reply-asks-or-tells", [
  { look: deckRow("Retry cap"), say: "Retry cap ended its turn on a question. Its row is yellow and pulses while it needs him" },
  { look: deckRow("Pin the store clock"), say: "Pin the store clock only told him something. Its row keeps its place and wears a small faint mark" },
  { hover: inside(deckRow("Trim the log lines"), role("img", "Unread reply")), say: "Unread reply. The second one, with no call" },
  { look: deckRow("Migration notes"), say: "Migration notes is idle, with no mark" },
  { key: "c", on: list, say: "c goes to the first row that needs him" },
  { press: inside(region(/^Flight control/), button(/^Timeline/)), say: "The Timeline, from the flight controls" },
  { look: inside(timeline, role("list", "Needs you")), say: "The lane that needs him pulses" },
  { look: inside(timeline, role("list", "Waiting")), say: "The lanes that told wear the same faint mark" },
  { press: deckRow("Retry cap"), say: "Enter or a press on the row opens the Session" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, at the foot of the thread" },
  { look: inside(needs, text(/Should the retry cap be five or ten/)), say: "The line that asks" },
]);

export { replyAsksOrTells as "reply-asks-or-tells" };
