// The Jobs list's filter chip has its own lane above the first row and no longer covers its buttons. Mock only. Over the `job-browser` scenario.

import { button, inside, region, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const list = region("Jobs");

const steps: Step[] = [
  toSessions,
  {
    hover: list,
    say: "Over the Jobs list, the filter comes up in a lane of its own above the first row",
  },
  {
    look: inside(
      list,
      button("Filter", {
        exact: false,
      }),
    ),
    say: "The first row's buttons are all clear of it",
  },
];

const filterChip = walk("job-browser", steps);

export { filterChip as "jobs-list-filter-chip" };
