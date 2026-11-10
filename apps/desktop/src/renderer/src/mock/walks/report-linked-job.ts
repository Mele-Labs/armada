// A reported bug and the Job started on it: the glyph on its row, the Job in the reader, and the report among the Job's issues. Over the `report-linked-job` scenario.

import { button, inside, place, region, role, walk } from "../walk";

const list = region("Issues", {
  exact: true,
});
const reader = region("Reported bug", {
  exact: true,
});

const reportLinkedJob = walk("report-linked-job", [
  {
    press: place("Issues"),
    say: "Issues, with All picked. Reports are drawn only under Reported",
  },
  {
    key: "L",
    on: list,
    say: "L opens the filter, where Reported is one more kind",
  },
  {
    press: role("menuitem", "Reported", {
      exact: true,
    }),
    say: "A report with a Job shows the Job's state beside its title",
  },
  {
    hover: inside(list, role("img", /^Stop a watch once its pull request merges:/)),
    say: "The glyph is the Job's own state. The tooltip names the Job",
  },
  {
    press: inside(list, button(/^A watch stayed live/)),
    say: "The reader shows the Job where Dispatch would be",
  },
  {
    press: inside(reader, button(/^Open the Job/)),
    say: "A press opens the Job",
  },
  {
    press: inside(
      region("Done", {
        exact: true,
      }),
      role("button", /^Done/),
    ),
    say: "Done, where the Job lists what it made and works on",
  },
  {
    hover: role("img", "Reported bug", {
      exact: true,
    }),
    say: "The Job lists the report it was started on, with the bug glyph",
  },
  {
    press: place("Issues"),
    say: "Issues again",
  },
  {
    key: "L",
    on: list,
    say: "L opens the filter, where Reported is one more kind",
  },
  {
    press: role("menuitem", "Reported", {
      exact: true,
    }),
    say: "Reported",
  },
  {
    press: inside(list, button(/^waiting_for refused/)),
    say: "A report with no Job",
  },
  {
    press: inside(
      reader,
      button("Dispatch", {
        exact: true,
      }),
    ),
    say: "Dispatch opens the drafted Job with the report as its brief. Send starts it",
  },
]);

export { reportLinkedJob as "report-linked-job" };
