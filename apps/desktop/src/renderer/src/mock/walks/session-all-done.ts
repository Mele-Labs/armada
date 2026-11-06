// A Session whose agent says it has nothing left: a card at the end of the thread. It lists
// the pull requests, merged first and each with its state and checks, and where there are none, the pages, files and Docs it made; each is a press.
// Keep going puts it away and puts the cursor in the message box; End job ends the Session, and its
// Artifacts stay to be read. Over the `session-all-done` and `session-all-done-artifacts` scenarios.
// Told twice each: wide, and below the breakpoint.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { NARROW, kit, toSessions } from "../sessions/walk-kit";

const callout = region("Nothing left to do");

function opened(title: RegExp): Step[] {
  const mine = inside(
    region("Jobs", {
      exact: true,
    }),
    button(title),
  );
  return [
    toSessions,
    {
      press: mine,
      say: "Opened",
    },
    {
      look: callout,
      say: "A card at the end of the thread",
    },
  ];
}

const prs = inside(callout, role("list", "Pull requests"));
const artifacts = inside(callout, role("list", "Artifacts"));

function keep(narrow: boolean): Step[] {
  const { artifacts, opened: inArtifacts, message } = kit(narrow);
  return [
    ...opened(/Pin the store clock/),
    {
      look: prs,
      say: "Each pull request with its state and checks",
    },
    {
      look: inside(callout, button("End job")),
      say: "End job under them",
    },
    ...inArtifacts([
      {
        look: inside(
          artifacts,
          region("Pull requests", {
            exact: true,
          }),
        ),
        say: "The Artifacts layer holds the pull requests too, with state and checks",
      },
    ]),
    {
      press: inside(callout, button("Keep going")),
      say: "Keep going puts it away",
    },
    {
      look: message,
      say: "The cursor is in the message box",
    },
  ];
}

const { artifacts: layer, opened: inLayer } = kit(false);

const made: Step[] = [
  ...opened(/Write up the store clock/),
  {
    look: artifacts,
    say: "The page and the Doc it made, and not the picture it looked at",
  },
  ...inLayer([
    {
      look: inside(
        layer,
        region("Files", {
          exact: true,
        }),
      ),
      say: "The picture is in the Artifacts layer, under Files",
    },
  ]),
];

function end(narrow: boolean): Step[] {
  const { artifacts, opened: inArtifacts } = kit(narrow);
  return [
    ...opened(/Pin the store clock/),
    {
      press: inside(callout, button("End job")),
      say: "End job ends the Job",
    },
    {
      look: role("img", "Ended"),
      say: "Ended",
    },
    ...inArtifacts([
      {
        look: inside(
          artifacts,
          region("Pull requests", {
            exact: true,
          }),
        ),
        say: "Its pull requests stay in Artifacts",
      },
    ]),
  ];
}

const wide = walk("session-all-done", keep(false));
const narrow = walk("session-all-done", keep(true), NARROW);
const wideMade = walk("session-all-done-artifacts", made);
const narrowMade = walk("session-all-done-artifacts", made, NARROW);
const wideEnd = walk("session-all-done", end(false));
const narrowEnd = walk("session-all-done", end(true), NARROW);

export {
  wide as "session-all-done",
  narrow as "session-all-done-narrow",
  wideMade as "session-all-done-artifacts",
  narrowMade as "session-all-done-artifacts-narrow",
  wideEnd as "session-all-done-end",
  narrowEnd as "session-all-done-end-narrow",
};
