// Main goes red on CI, and the owner's part in it. The hub's head is a mark while main is green and
// a red frame while it is not, naming the Check, the test and the merge, each a link. A Job that
// merged its own pull request takes the red itself. Otherwise the owner chooses: send the work back
// to an earlier Job, the culprit's first, or dispatch a new one. Each ends with the fix landing.
// Pull requests whose `ci` is red only because main is read as waiting on the fix. Each `later` is
// time passing: the scenario publishes its next moment.

import { button, dialog, inside, region, role, text, walk } from "../walk";

const MERGE = region("Merge line");
const RED = inside(MERGE, role("status", "Main is red"));
const PULLS = inside(MERGE, role("list", "Open pull requests"));
const LOG = dialog("Check log");
const SEND = dialog("Send the work back to a Job");
const DISPATCH = dialog("Dispatch a Job to fix main");

const CACHE = "Cache the manifest read between dispatches";
const DEBOUNCE = "Debounce the Job Board's resize handler";
const FIX = "Fix components_test on main";

const ROW = (title: string) => role("option", new RegExp(title));

export const mainGoesRed = walk("main-red-hub", [
  // Main is green.
  { hover: inside(MERGE, role("img", "Main is green")), say: "Main is green: a mark beside the heading, its tooltip the only words" },
  { look: PULLS, say: "Every open pull request in the repository, with how its ci stands" },
  { look: inside(PULLS, role("listitem", "fleet/pause-markers, ci running")), say: "A ci still running pulses" },
  { later: inside(PULLS, role("listitem", /^armada\/60-cache, ci passed/)), say: "This Job's pull request is about to merge" },

  // The Job's own pull request turned main red. It was watching its landing and has taken the red.
  { look: RED, say: "Main is red: a red band titles the frame, which stays when the panel is folded" },
  { hover: inside(RED, button("screens_test")), say: "Check: the failing one, labelled; a press opens its log" },
  { look: inside(RED, text(/^manifest-read\.test\.ts/)), say: "Test: the failing one, labelled" },
  { look: inside(RED, role("link", "#1812")), say: "Broke in: the pull request that merged" },
  { look: inside(RED, text("armada/60-cache")), say: "And its branch" },
  { press: inside(RED, button("screens_test")), say: "The Check's log opens from the head, as a strip's segment does" },
  { look: inside(LOG, text("AssertionError: expected 'cached' to be 'changed'")), say: "Main's run, whole" },
  { press: inside(LOG, button("Close")), say: "Put it away" },
  { hover: inside(RED, role("img", "Working on it")), say: "Fixing: the Job that merged it took it itself, and nobody is asked" },
  { look: inside(RED, button(CACHE)), say: "Its title opens the Job" },
  { look: inside(PULLS, role("listitem", /^studio\/zone-proposal, ci red because main is/)), say: "These ci runs are red because main is: waiting on the fix" },
  { hover: inside(PULLS, role("img", /waiting on the fix: Cache the manifest read between dispatches/)), say: "Hover names the fix it waits on" },
  { look: inside(PULLS, role("listitem", "fleet/pause-markers, ci failed")), say: "A branch's own failure stays a failure" },
  { hover: role("img", /^Fixing main: screens_test failed after #1812/), say: "On the Board the Job's row carries a hammer beside its badge, pulsing" },
  { press: ROW(CACHE), say: "Open the Job" },
  { look: role("heading", "Fixing main"), say: "Its lead: what it took" },
  { look: text("manifest-read.test.ts > reads the file again once it changes · #1812"), say: "The failing test and the pull request that turned main red" },
  { hover: role("img", /^Fixing main/), say: "The same mark beside the header's badge" },
  { press: button("Read the log"), say: "The log is attached: the same one the head opens" },
  { look: inside(LOG, text("AssertionError: expected 'cached' to be 'changed'")), say: "Main's run, from the Job too" },
  { press: inside(LOG, button("Close")), say: "Put it away" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },

  // Its fix lands.
  { later: role("img", /^Fixing main/), say: "Its fix reaches main" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Main is green again: the head is a mark once more" },
  { look: inside(PULLS, role("listitem", "studio/zone-proposal, ci passed")), say: "What waited on the fix has its own ci back" },
  { press: button("Expand Done"), say: "A Job that finished is under Done" },
  { hover: role("img", "Fixed main in #1821"), say: "The Job that fixed it says so beside its badge" },
  { press: ROW(CACHE), say: "Open it" },
  { hover: role("img", "Fixed main in #1821"), say: "Its header says the same: the pull request that fixed it" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },

  // A person merged the Job's pull request on the forge: nothing was watching it.
  { later: inside(MERGE, role("img", "Main is green")), say: "A person merges this Job's pull request on the forge" },
  { look: RED, say: "Main is red again, and this time no Job has it" },
  { look: inside(RED, button("desktop_test")), say: "The Check, the test, the merge and its branch, as before" },
  { look: inside(RED, role("link", "#1816")), say: "The pull request that merged" },
  { look: inside(RED, button("Dispatch a new Job")), say: "Two ways: a new Job" },
  { press: inside(RED, button("Send back to a Job")), say: "Or an earlier one" },
  { look: inside(SEND, role("list", "Attached")), say: "What goes with it: the Check, the test, the log and the pull request" },
  { look: inside(SEND, role("img", "Merged #1816")), say: "The Job whose pull request it was comes first, marked" },
  { press: inside(SEND, role("radio", new RegExp(`^${DEBOUNCE}`))), say: "Choose it" },
  { press: inside(SEND, button("Send back", { exact: true })), say: "Send the work back" },
  { later: inside(RED, button("Send back to a Job")), say: "Fleet takes it up" },
  { look: inside(RED, button(DEBOUNCE)), say: "The Job has it: the two ways are gone" },
  { hover: inside(RED, role("img", "Working on it")), say: "Working on it" },
  { press: inside(RED, button(DEBOUNCE)), say: "Open the Job" },
  { look: role("heading", "Fixing main"), say: "Its lead is the same as when it took the red itself" },
  { look: text("resources-poll.test.ts > a reading that stops polls nothing · #1816"), say: "The test and the pull request, attached" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },
  { later: inside(RED, button(DEBOUNCE)), say: "Its fix reaches main" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Green again" },
  { look: role("img", "Fixed main in #1822"), say: "The Job says it fixed it" },

  // A person's own pull request turned it red: no Job to send it to first, and the owner dispatches one.
  { later: inside(MERGE, role("img", "Main is green")), say: "A person merges their own pull request" },
  { look: inside(RED, text("nick/theme-tokens")), say: "Main is red from a person's pull request: no Job of its own" },
  { press: inside(RED, button("Dispatch a new Job")), say: "Dispatch a new Job" },
  { look: inside(DISPATCH, role("list", "Attached")), say: "Prefilled: the Check, the test, the log and the pull request" },
  { look: inside(DISPATCH, role("textbox", "Brief")), say: "A brief of bare facts, there to edit" },
  { press: inside(DISPATCH, button("Dispatch", { exact: true })), say: "Dispatch it" },
  { later: inside(RED, button("Dispatch a new Job")), say: "Fleet starts the Job" },
  { look: inside(RED, button(FIX)), say: "The new Job has it" },
  { press: inside(RED, button(FIX)), say: "Open the Job" },
  { look: role("heading", "Fixing main"), say: "Running, with the test and the log attached" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },
  { later: inside(RED, button(FIX)), say: "Its fix reaches main" },
  { look: inside(MERGE, role("img", "Main is green")), say: "Green" },
  { later: role("img", "Fixed main in #1823"), say: "The Job that fixed it says so" },

  // CI outside Armada: the failing job maps to no Manifest Check. Armada goes on what the forge reports.
  { look: RED, say: "Main is red again, from CI that Armada has no Check for" },
  { look: inside(RED, text("CI job")), say: "CI job, not Check: the failing job's name as the forge reports it" },
  { press: inside(RED, button("test-all")), say: "Its log, from the forge, opens from it" },
  { look: inside(LOG, text("Process completed with exit code 1")), say: "The forge's log" },
  { press: inside(LOG, button("Close")), say: "Put it away" },
  { look: inside(RED, role("link", "#1814")), say: "Broke in: still the pull request that merged. No Test row: no test could be read out of the log" },
  { look: inside(RED, button("Name the zone a read-in lands in")), say: "Fixing: the Job behind it took it as before, with the CI job and its log attached" },
  { press: inside(RED, button("Name the zone a read-in lands in")), say: "Open the Job" },
  { look: role("heading", "Fixing main"), say: "Its lead names the CI job and the pull request" },
  { press: button("Overview", { exact: true }), say: "Back to the Board" },
  { later: inside(RED, button("Name the zone a read-in lands in")), say: "Its fix reaches main" },
  { look: role("img", "Fixed main in #1824"), say: "The Job that fixed it says so" },
]);
