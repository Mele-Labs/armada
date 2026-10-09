// Main is red, and other work has landed on it with CI still running. The red is held until that run
// ends: the band turns caution, says new checks are running on main and names the pull request, and
// offers no way to hand the red to a Job, because the run may already have fixed it. Recently landed
// shows each merge's own run on main, apart from the pull request's checks it passed before it merged.
// A run that ends green clears the band, even while newer checks run behind it; one that ends red on the same job brings the red back with
// its buttons. Each `later` is time passing: the scenario publishes its next moment.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const HELD = inside(MERGE, role("status", "New checks are running on main"));
const LANDED = inside(MERGE, role("list", "Recently landed"));
const LOG = dialog("Check log");

export const mainChecksRunning = walk("main-checks-running", [
  { press: role("button", "Merge line", { exact: true }), say: "The merge line" },
  // Main is red at #1812, and nothing is running on top of it.
  { look: RED, say: "Main is red: nothing newer is running, so the red band offers its two ways" },
  { look: inside(RED, button("Dispatch a new Job")), say: "A new Job, or an earlier one, as before" },
  { hover: inside(LANDED, role("img", "ci failed on main")), say: "Recently landed: the run on main for #1812's merge commit, failed, apart from its pull request's own checks" },
  { press: inside(LANDED, button("ci failed on main, open its log")), say: "Its failed job's log opens from the row, as the band's does" },
  { look: inside(LOG, text("AssertionError: expected 2 to be 1")), say: "That run's log, whole" },
  { press: inside(LOG, button("Close")), say: "Put it away" },
  { look: inside(LANDED, role("img", "ci passed on main")), say: "Older merges show their run passed" },

  // #1852 merges and its run starts.
  { later: RED, say: "#1852 merges, and CI starts on main" },
  { look: HELD, say: "The band is caution: new checks are running on main, naming the pull request" },
  { look: inside(HELD, role("link", "#1852")), say: "Its link opens the pull request" },
  { look: inside(HELD, button("desktop_test")), say: "The red's rows stay under it: the Check, the test, the merge" },
  { look: inside(HELD, role("link", "#1812")), say: "Broke in: the pull request that merged the red" },
  { hover: inside(MERGE, role("img", "Main is red, new checks are running")), say: "The mark beside the heading is caution too" },
  { look: inside(LANDED, role("img", "ci running on main")), say: "Recently landed shows #1852's run going, pulsing" },
  { press: inside(HELD, button("desktop_test")), say: "The red's log still opens: it is the failed commit's run" },
  { press: inside(LOG, button("Close")), say: "Put it away" },

  // #1852's run ends green while #1853's is going.
  { later: HELD, say: "#1852's run finishes green, and #1853 has merged behind it" },
  { hover: inside(MERGE, role("img", "Main is green, new checks are running")), say: "The newest finished run is green, so the red is gone, and the mark says checks are still running" },
  { look: inside(LANDED, role("img", "ci running on main")), say: "#1853's run is going, pulsing in Recently landed" },
  { look: inside(LANDED, role("img", "ci passed on main")), say: "#1852's run on main passed" },
  { later: inside(MERGE, role("img", "Main is green, new checks are running")), say: "#1853's run finishes green" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Plain green: nothing was handed to a Job" },

  // #1855 breaks main again, and #1856 merges on top of it.
  { later: inside(MERGE, role("img", "Main is green")), say: "#1855 merges and breaks a job" },
  { look: RED, say: "A red naming the merge that started it, and its buttons" },
  { look: inside(RED, role("link", "#1855")), say: "Broke in: #1855, not #1812" },
  { later: RED, say: "#1856 merges and its run starts" },
  { look: inside(HELD, role("link", "#1856")), say: "Held again, now naming #1856" },
  { look: inside(HELD, role("link", "#1855")), say: "Under it the red is still #1855's" },
  { look: inside(LANDED, role("img", "ci running on main")), say: "And its run on main is going" },

  // The run ends red on the same job.
  { later: HELD, say: "The run finishes red, on the job that was already failing" },
  { look: RED, say: "The red band is back" },
  { look: inside(RED, button("Dispatch a new Job")), say: "With its buttons" },
  { look: inside(RED, role("link", "#1855")), say: "Still #1855's red" },
  { look: inside(LANDED, role("img", "ci failed on main")), say: "#1856's run on main failed, with its log a press away" },
]);
