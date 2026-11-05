// The split the owner decided: what he answers stays on the Needs you card, what he reads is on the sheet. A Job with only a question has the card and no Review row; a Job with both
// has the question and a Review row on the card, which opens a sheet that lists only the plan, sketch, draft and document. A walk is a row of the card that brings its tab forward.
// Over the `job-review-sheet` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const jobs = region("Jobs", {
  exact: true,
});
const needs = region("Needs you");
const sheet = dialog("Retry backoff cap");
const list = inside(sheet, role("group", "Waiting on you"));

const wired = walk("job-review-sheet", [
  toSessions,
  {
    press: inside(jobs, button(/Name the cache key/)),
    say: "A Job with a question and a Job it started, and nothing to review",
  },
  {
    look: inside(needs, button(/Open What should the cache key include/)),
    say: "The question is on the card",
  },
  {
    look: inside(
      needs,
      button("Start", {
        exact: true,
      }),
    ),
    say: "So is the Job it started, with its Start",
  },
  {
    look: inside(
      needs,
      button("Needs you", {
        exact: true,
      }),
    ),
    say: "Nothing to review, so the list has no row for it",
  },
  {
    press: inside(needs, button(/Open What should the cache key include/)),
    say: "The question is answered in place",
  },
  {
    press: inside(needs, role("radio", /The path and the commit/)),
    say: "Its own options",
  },
  {
    press: inside(needs, button("Answer")),
    say: "Answered, and the card has the Start left",
  },
  {
    press: inside(jobs, button(/Retry backoff cap/)),
    say: "A Job with a question, a command, a walk and four things to review",
  },
  {
    look: inside(needs, button(/Open Keep the old fixture/)),
    say: "The question is on the card",
  },
  {
    look: inside(
      needs,
      button("Review", {
        exact: true,
      }),
    ),
    say: "What he reads is one row in the same list",
  },
  {
    press: inside(
      needs,
      button("Review", {
        exact: true,
      }),
    ),
    say: "The row opens the sheet",
  },
  {
    look: inside(list, button(/Plan v2/)),
    say: "It lists the plan, sketch, draft and document, and not the question, the command or the walk",
  },
  {
    press: inside(sheet, button("Close")),
    say: "Close the sheet",
  },
  {
    look: inside(needs, button(/Open Backoff table/)),
    say: "The walk is a row of the card, and a press brings its tab forward",
  },
  {
    press: inside(jobs, button(/Draw the retry curve/)),
    say: "A Job with only something to review",
  },
  {
    press: inside(
      needs,
      button("Needs you", {
        exact: true,
      }),
    ),
    say: "The card has nothing to fold, so the band opens the sheet",
  },
  {
    look: inside(dialog("Draw the retry curve"), button(/^Approve/)),
    say: "Approve names the sketch",
  },
]);

export { wired as "job-review-sheet-wired" };
