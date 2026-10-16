// The calls a Session waits on him for, pinned under its thread: they shrink to their band as he scrolls the thread up, open over the
// whole thread from a press on the band, and close with the thread where he left it. Over the `pinned-calls` scenario.

import { deckRow, role, walk } from "../walk";

const calls = role("region", "Pinned calls");
const thread = role("region", "Thread", { exact: true });
const band = role("button", /^Plan to approve, Question/);

export const pinnedCallsScroll = walk("pinned-calls", [
  { press: deckRow("Fold Jobs into Sessions"), say: "The Session is open on a long thread" },
  { look: calls, say: "The question and the plan are pinned under the thread, whole while the thread is at its end" },
  { key: "PageUp", on: thread, say: "Scroll the thread up, and the pinned calls shrink with it until only the band is left" },
  { look: band, say: "The band names the plan, with the question as a chip. Press it to open the calls over the thread" },
  { press: band, say: "Open over the thread, to scroll inside" },
  { press: band, say: "Closed again" },
  { look: thread, say: "The thread is where he left it" },
]);
