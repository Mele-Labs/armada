// Runs of one Check that sit side by side in a Session's thread are one row: the latest run's mark and
// the Check's name. Opened, each run is a line with what it was narrowed to. A different Check between
// them starts a fold of its own, and the run still out ends the thread.

import { button, inside, place, region, walk } from "../walk";

const THREAD = region("Thread");

export const foldRepeatedCheckRuns = walk("fold-repeated-check-runs", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button("Session check folds")), say: "A Job whose agent kept running `test`" },
  { look: inside(THREAD, button("Check test, latest failed")), say: "The runs of `test` fold into one row, showing the latest" },
  { look: inside(THREAD, button("Check desktop:typecheck, passed")), say: "A different Check between them ends the fold" },
  { hover: inside(THREAD, button("Check test, latest running")), say: "The runs after it fold again, the latest still out: the mark pulses" },
  { press: inside(THREAD, button("Check test, latest failed")), say: "Opening the first fold" },
  { look: inside(THREAD, button("Check test store::reads_the_last_row, failed")), say: "Each run, with the filter it was given" },
  { look: inside(THREAD, button("Check test --changed fleet, ipc, failed")), say: "Or the crates --changed narrowed to" },
  { look: inside(THREAD, button("Check test, passed")), say: "A run that ran whole names nothing" },
]);
