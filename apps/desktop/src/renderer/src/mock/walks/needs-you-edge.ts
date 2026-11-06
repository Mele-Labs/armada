// Needs you, the box under it and the thread's cards share one edge in a Job's pane: the band is as wide as the box, closed and
// open, and a document's card reaches the edge his bubble reaches. Over the `thread-states` scenario.

import { deckRow, inside, region, role, walk } from "../walk";

const thread = region("Thread");
const band = role("button", /^Needs you/);
const box = role("textbox", "Message");

const needsYouEdge = walk("thread-states", [
  {
    press: deckRow("Staging migration"),
    say: "A Job waiting on an answer",
  },
  {
    look: band,
    say: "Needs you stands on the box, as wide as it",
  },
  {
    look: box,
    say: "The box under it, on the same two edges",
  },
  {
    press: band,
    say: "Opened, the panel keeps that width",
  },
  {
    press: band,
    say: "Folded again",
  },
  {
    press: deckRow("Demo marketing video"),
    say: "A Job with a document waiting",
  },
  {
    look: inside(thread, role("article", /^Document: Video script/)),
    say: "Its card reaches the edge the box and his bubble reach",
  },
]);

export { needsYouEdge as "needs-you-edge" };
