// Take "One bar" of a question in Needs you: its options are quick bar chips inside the band, and the bar over the box draws nothing until it is answered. Over `band-one-bar`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const needs = region("Needs you");
const quick = role("group", "Quick replies");

const oneBar = walk("band-one-bar", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "The question's options are chips in the band, ⌘1 to ⌘3, and no bar over the box" },
  { press: inside(needs, button("Record the plan", { exact: true })), say: "A chip answers the question" },
  { look: inside(thread, text(/Going with Record the plan/)), say: "His answer is in the thread" },
  { look: inside(quick, button("Land it")), say: "The band is gone and the bar is back" },
]);

export { oneBar as "band-one-bar" };
