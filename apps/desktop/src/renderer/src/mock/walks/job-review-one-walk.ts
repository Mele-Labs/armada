// The sheet takes two or more things to read. One walk is a row on the Needs you card that brings its tab forward, and one sketch is its card in the thread, with no band at all.
// Two open the sheet from the band. Over the `job-review-sheet` scenario.

import { button, dialog, inside, region, role, tab, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const jobs = region("Jobs", {
  exact: true,
});
const needs = region("Needs you");

const oneWalk = walk("job-review-sheet", [
  toSessions,
  {
    press: inside(jobs, button(/Check the usage page/)),
    say: "A Job with one walk",
  },
  {
    press: inside(
      needs,
      button("Needs you", {
        exact: true,
      }),
    ),
    say: "The band opens the card and no sheet",
  },
  {
    press: inside(needs, button(/^Open Usage page/)),
    say: "The walk is one row, and a press brings its tab forward",
  },
  {
    look: tab("Usage page"),
    say: "The walk's tab",
  },
  {
    press: inside(jobs, button(/Sketch the cap/)),
    say: "A Job with one sketch: its card is in the thread, with no band",
  },
  {
    press: inside(jobs, button(/Draw the retry curve/)),
    say: "A Job with two things to review",
  },
  {
    press: inside(
      needs,
      button("Needs you", {
        exact: true,
      }),
    ),
    say: "The band opens the sheet",
  },
  {
    look: inside(dialog("Draw the retry curve"), role("group", "Waiting on you")),
    say: "The sheet lists both",
  },
]);

export { oneWalk as "job-review-one-walk" };
