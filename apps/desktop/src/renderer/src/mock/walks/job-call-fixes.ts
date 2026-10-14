// A Job that went wrong, fixed from its call: the card shows the failure itself and offers the fixes, so
// the owner need not open the Job. A Check failed (Retry, Skip, Skip all), a Drone gone quiet (Retry step,
// and words that redirect it), a ruling that failed (Retry step) and a Job in another's files. Over
// `job-call-fixes`.

import { button, inside, region, role, text, walk } from "../walk";

const tray = role("group", "Calls waiting");
const check = region(/^Check failed: Cap the retry backoff/);
const stuck = region(/^Drone stuck: Pin the store clock/);
const ruled = region(/^Failed ruling: Retire sleep calls/);
const clash = region(/^Overlap: Shorten the reconnect wait/);

const jobCallFixes = walk("job-call-fixes", [
  { press: button(/^Check failed: Cap the retry backoff/), say: "A call in the tray, opened with a press" },
  { look: check, say: "A Check failed: which Check, the tests that failed by name, and the end of its output" },
  { look: inside(check, role("list", "Failing tests")), say: "The failing tests, by name" },
  { press: button(/caps_at_thirty_seconds/), say: "A test pressed finds its line in the output and marks it" },
  { look: inside(check, text("left: 64s")), say: "The output scrolls from its last lines" },
  { hover: button(/2 Skip$/), say: "Skip says it ends the Check as skipped, not passed" },
  { look: button(/Skip all/), say: "Skip all is there because more than one Check is unfinished" },
  { key: "t", on: region("Dashboard"), say: "Message opens the box, which is shut until then" },
  { look: inside(check, role("textbox", "Message")), say: "Words here go to the Job's Drone" },
  { press: button(/Retry/), say: "One press on Retry, held while Fleet answers" },
  { press: inside(tray, button(/^Drone stuck: Pin the store clock/)), say: "The next call, from the tray" },
  { look: stuck, say: "A Drone gone quiet: how long, and what it last did" },
  { hover: inside(stuck, role("img", "Quiet for 47 minutes")), say: "The figure names itself on hover" },
  { look: button(/Retry step/), say: "Retry step starts the step again" },
  { key: "t", on: region("Dashboard"), say: "Message opens the box" },
  { type: "Read the failing test first, then pin the clock in the setup\n", into: inside(stuck, role("textbox", "Message")), say: "Words redirect the Drone, and the card leaves" },
  { press: inside(tray, button(/^Failed ruling: Retire sleep calls/)), say: "The next call, from the tray" },
  { look: ruled, say: "A ruling that failed: the attempt, what brought it, and the Judge's reason" },
  { press: button(/Retry step/), say: "One press on Retry step" },
  { press: inside(tray, button(/^Overlap: Shorten the reconnect wait/)), say: "The next call, from the tray" },
  { look: clash, say: "Another Job in the same files: what they share, and the other Job to open" },
  { look: button(/Open the other Job/), say: "Open the other Job is the primary here" },
]);

export { jobCallFixes };
