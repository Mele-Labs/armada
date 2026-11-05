// The message box grows with what is typed, up to half the height of its panel, and then scrolls.
// Told on a Session's composer.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, walk } from "../walk";
import type { Step } from "../walk";

const lines = (count: number) =>
  Array.from(
    {
      length: count,
    },
    (_, index) => `Line ${index + 1} of the message`,
  ).join("\n");

function steps(): Step[] {
  const { message } = kit(false);
  const sessions = region("Jobs", {
    exact: true,
  });
  return [
    {
      press: inside(sessions, button("CI timeout hunt")),
      say: "A Job, with its message box at rest",
    },
    {
      type: lines(5),
      into: message,
      say: "Five lines, and the box has grown with them",
    },
    {
      type: lines(40),
      into: message,
      say: "Forty lines: the box stops at half the panel and scrolls inside",
    },
    {
      type: "",
      into: message,
      say: "Cleared, and the box is back to its resting height",
    },
  ];
}

const keeping: Step[] = [
  {
    press: inside(
      region("Jobs", {
        exact: true,
      }),
      button("CI timeout hunt"),
    ),
    say: "A Job",
  },
  {
    type: "Half a thought before I leave",
    into: kit(false).message,
    say: "Something typed and not sent",
  },
  {
    press: place("Cockpit"),
    say: "Off to the Cockpit",
  },
  {
    press: place("Cockpit"),
    say: "And back to the Jobs",
  },
  {
    press: inside(
      region("Jobs", {
        exact: true,
      }),
      button("CI timeout hunt"),
    ),
    say: "The same Job",
  },
  {
    look: kit(false).message,
    say: "The message box has what was typed, with the caret at its end",
  },
];

const session = walk("session-thread-polish", [
  toSessions,
  ...steps(),
]);

const kept = walk("session-thread-polish", [
  toSessions,
  ...keeping,
]);

export {
  kept as "session-composer-keeps",
  session as "session-composer-grows",
};
