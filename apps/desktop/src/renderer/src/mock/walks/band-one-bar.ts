// Take "One bar" of a question in Needs you: its options are the quick bar's first chips and the band keeps its words. Over `band-one-bar`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const quick = role("group", "Quick replies");

const oneBar = walk("band-one-bar", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "Needs you holds the question's words and no options" },
  { look: inside(quick, button("Record the plan")), say: "The options are the bar's first chips, ⌘1 to ⌘3" },
  { press: inside(quick, button("Record the plan")), say: "A chip answers the question, as the band would" },
  { look: inside(thread, text(/Going with Record the plan/)), say: "His answer is in the thread" },
  { look: inside(quick, button("Land it")), say: "The band is gone and the bar holds only his own lines" },
]);

export { oneBar as "band-one-bar" };
