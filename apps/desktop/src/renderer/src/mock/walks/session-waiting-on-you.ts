// A Session that waits on the person: its row has a hand, the Needs you panel on its
// composer is up with the question, and the page item is a walk on the sheet the Review row opens. Over the `session-waiting-on-you` scenario. Told twice: wide, and
// below the breakpoint.

import { button, dialog, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const sessions = region("Jobs", {
    exact: true,
  });
  const needs = region("Needs you");
  const mine = inside(sessions, button(/Pin the store clock/));
  const sheet = dialog("Pin the store clock");
  const page = inside(
    inside(sheet, role("group", "Waiting on you")),
    button(/Look at the findings page/),
  );
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
      look: inside(needs, button("Open Which clock?")),
      say: "The question is a row on the card",
    },
    {
      press: inside(
        needs,
        button("Review", {
          exact: true,
        }),
      ),
      say: "The page item is a walk, so it is on the sheet Review opens",
    },
    {
      look: page,
      say: "Its walk plays in the sheet",
    },
    {
      key: "Escape",
      on: page,
      say: "Esc closes the sheet",
    },
    {
      look: tab("Look at the findings page", {
        exact: true,
      }),
      say: "The walk is also a tab of the Job",
    },
    {
      press: button("Close Look at the findings page"),
      say: "Closed with its ×. The walk is still owed",
    },
    {
      press: inside(needs, button("Open Which clock?")),
      say: "The question opens on the card",
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
