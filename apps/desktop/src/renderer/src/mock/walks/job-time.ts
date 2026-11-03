// The Usage page's Time view: switch to it, read the hours, then open the slowest Job's split and pick a build.
// Keys on the page: v switches between tokens and time. In Time, ↓ j and ↑ k move down the Jobs, ↵ o opens one,
// ← → h l step the hours and Esc clears the hour, then the Job, then the build. c moves focus to the Builds list, where
// ↓ j and ↑ k move and ↵ o pick.

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
    look: region("Hour by hour"),
    say: "The four figures by the hour: Model, Tools and Pull request hold while Checks steps up at a build's marker",
  },
  {
    key: "ArrowDown",
    on: JOBS,
    say: "↓ moves the cursor to the slowest Job",
  },
  {
    key: "Enter",
    on: JOBS,
    say: "↵ opens its split",
  },
  {
    look: role("list", "Job split"),
    say: "Most time first; the usual for its workflow shows where it runs far over",
  },
  {
    key: "Escape",
    on: role("dialog"),
    say: "Esc closes it",
  },
  {
    press: role("button", /^7ac3d52/),
    say: "Press the flag where Checks stepped up, or its row 7ac3d52 in Builds: the Jobs and the split narrow to that build, and its span is shaded. Esc clears it",
  },
  {
    look: region("Split", {
      exact: true,
    }),
    say: "The split is that build's",
  },
  {
    press: tab("Tokens", {
      exact: true,
    }),
    say: "Tokens keeps the build and shades its span on Burn per hour: burn per turn holds across the builds",
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
