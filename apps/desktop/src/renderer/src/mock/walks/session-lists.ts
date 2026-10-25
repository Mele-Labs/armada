// A Session's reply with a numbered list and a bullet list: both sit in from the column's edge, so the numbers and
// bullets do not stand against it. Over the `session-loadout` scenario, whose third reply holds the lists.

import { inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const sessions = region("Jobs", { exact: true });

const sessionLists = walk("session-loadout", [
  toSessions,
  { press: inside(sessions, role("button", /Why did the mechanic skip/)), say: "The Job" },
  { look: text("Two things let the push through:"), say: "The reply's numbered list and its bullets, in from the column's edge" },
]);

export { sessionLists as "session-lists" };
