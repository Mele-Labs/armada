// A Session that is quiet or ended, forked. One from a terminal whose mod stopped asking has its conversation
// and a Fork in its head's More menu, and no message box; a live one has the box and no Fork. Fork starts a new
// Session with the same title and the old conversation, which opens at once with a row linking back,
// and the old one now links forward. Told twice, wide and below the breakpoint.

import { kit, NARROW, toSessions } from "../sessions/walk-kit";
import { deckRow, button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

function steps(narrow: boolean): Step[] {
  const { message, thread } = kit(narrow);
  const sessions = region("Jobs", { exact: true });
  const filter = inside(sessions, button(/^Filter/));
  const scope = (name: string) => role("menuitem", name, { exact: true });
  const more = inside(role("region", /^Job: /), button("More"));
  const fork = role("menuitem", "Fork", { exact: true });
  return [
    { look: inside(sessions, button("Fix the flaky store test")), say: "A live Job is in the list" },
    { press: inside(sessions, button("Fix the flaky store test")), say: "It opens" },
    { look: message, say: "It takes a message" },
    { press: filter, say: "The list's Filter holds Quiet and Ended" },
    { press: scope("Quiet"), say: "Quiet is a view of its own" },
    { look: inside(sessions, button("Why the reader drops lines")), say: "A terminal Job whose mod is not asking is under Quiet" },
    { press: filter, say: "Ended" },
    { press: scope("Ended"), say: "One that ended is under Ended, apart from it" },
    { look: inside(sessions, button("Trim the retry loop")), say: "Ended Jobs" },
    { press: filter, say: "Back to Quiet" },
    { press: scope("Quiet"), say: "Quiet" },
    { press: inside(sessions, button("Why the reader drops lines")), say: "A terminal Job whose mod stopped asking" },
    { look: inside(thread, text("It splits on newlines")), say: "Its conversation is still there" },
    { press: more, say: "In place of the message box, Fork is in the head's More menu" },
    { look: fork, say: "Fork" },
    { press: fork, say: "A new Job starts with this conversation" },
    { look: region("Job s-01FORK00"), say: "It opens at once, with the old title" },
    { look: role("group", "Forked from Why the reader drops lines"), say: "Its thread opens on one quiet row, closed" },
    { press: role("button", "Forked from Why the reader drops lines", { exact: true }), say: "Pressed, it opens onto the old conversation" },
    { look: inside(role("group", "Forked from Why the reader drops lines"), text("It splits on newlines")), say: "What was said before the fork, to read" },
    { look: message, say: "And it takes a message" },
    { press: deckRow("Why the reader drops lines"), say: "The Job it came from opens from the list" },
    { press: more, say: "Still dead, and still offers Fork" },
    { look: fork, say: "Fork" },
  ];
}

const wide = walk("session-fork", [toSessions, ...steps(false)]);
const narrow = walk("session-fork", [toSessions, ...steps(true)], NARROW);

export { wide as "session-fork", narrow as "session-fork-narrow" };
