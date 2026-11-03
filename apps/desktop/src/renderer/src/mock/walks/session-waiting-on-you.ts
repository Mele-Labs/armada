// A Session that waits on the person: its row has a hand, the Needs you panel on its
// composer is up with the question and the items, a press on the page item opens its walk, and the question is its first part. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", {
    exact: true,
  });
  const needs = region("Needs you");
  const mine = inside(sessions, button(/Pin the store clock/));
  return [
    {
      hover: inside(sessions, role("img", /^Waiting on you/)),
      say: "A hand on its row, and on hover what it waits on",
    },
    {
      press: mine,
      say: "Opened",
    },
    {
      press: inside(needs, button("Open Look at the findings page")),
      say: "A page item's row brings its walk to the front of the Job's panes",
    },
    {
      look: tab("Look at the findings page", {
        exact: true,
      }),
      say: "The walk is a tab of the Job",
    },
    {
      press: button("Close Look at the findings page"),
      say: "Closed with its ×. The walk is still owed",
    },
    {
      press: inside(needs, button("Open Which clock?")),
      say: "The question is the first row",
    },
    {
      look: role("article", "Waiting on you"),
      say: "Opened, it is answered in the panel",
    },
  ];
}

const wide = walk("session-waiting-on-you", [
  toSessions,
  ...steps(),
]);
const narrow = walk(
  "session-waiting-on-you",
  [
    toSessions,
    ...steps(),
  ],
  NARROW,
);

export { wide as "session-waiting-on-you", narrow as "session-waiting-on-you-narrow" };
