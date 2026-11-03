// The Usage page's Time view: switch to it, read the weeks, then open the slowest Job's split.
// Keys on the page: v switches between tokens and time. In Time, ↓ j and ↑ k move down the Jobs, ↵ o opens one,
// ← → h l step the weeks and Esc clears the week and then the Job.

import { button, deckRow, inside, place, region, role, tab, viaPalette, walk } from "../walk";

const JOBS = role("listbox", "Slowest Jobs", {
  exact: true,
});

export const jobTime = walk("usage/time", [
  ...viaPalette("Usage", "Usage, by name"),
  {
    press: tab("Time", {
      exact: true,
    }),
    say: "Time sits beside Tokens; v switches between them",
  },
  {
    look: region("Week by week"),
    say: "The median Job by week: Model holds still while Checks and Pull request grow",
  },
  {
    key: "ArrowDown",
    on: JOBS,
    say: "↓ moves the cursor to the slowest Job",
  },
  {
    key: "ArrowDown",
    on: JOBS,
    say: "↓ again, to the next one",
  },
  {
    key: "Enter",
    on: JOBS,
    say: "↵ opens its split",
  },
  {
    look: region("Job split"),
    say: "Most time first; the usual for its workflow shows where it runs far over",
  },
  {
    press: place("Cockpit"),
    say: "The Cockpit",
  },
  {
    press: deckRow("Jobs -> Session Merger"),
    say: "The same Job",
  },
  {
    press: inside(
      region("Pane: Thread", {
        exact: true,
      }),
      button("Add tab"),
    ),
    say: "Add a tab to the Job",
  },
  {
    press: role("menuitem", "System", {
      exact: true,
    }),
    say: "System holds what the Job runs",
  },
  {
    look: inside(
      region("System", {
        exact: true,
      }),
      region("Time", {
        exact: true,
      }),
    ),
    say: "The same split, on the Job",
  },
]);
