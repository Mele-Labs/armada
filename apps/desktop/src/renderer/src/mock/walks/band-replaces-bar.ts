// A question in Needs you: the quick bar is not drawn while the band is up. Over `band-replaces-bar`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Thread");
const needs = region("Needs you");
const quick = role("group", "Quick replies");

const replaces = walk("band-replaces-bar", [
  toSessions,
  { press: deckRow("Header Font Sizes"), say: "Needs you is up and the quick bar is not drawn" },
  { press: inside(needs, role("radio", "Record the plan", { exact: true })), say: "The options are answered in the band" },
  { press: inside(needs, button("Answer")), say: "Answer sends the pick" },
  { look: inside(thread, text(/Going with Record the plan/)), say: "His answer is in the thread" },
  { look: inside(quick, button("Land it")), say: "The band is gone and the bar is back" },
]);

export { replaces as "band-replaces-bar" };
