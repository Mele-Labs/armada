// A Job that went wrong, fixed from its call: the card shows the failure itself and offers the fixes, so
// the owner need not open the Job. A Check failed (Retry, Skip, Skip all), a Drone gone quiet (Retry step,
// and words that redirect it), a ruling that failed (Retry step) and a Job in another's files. Over
// `job-call-fixes`.

import { button, inside, region, role, text, walk } from "../walk";

const check = region(/^Check failed: Cap the retry backoff/);
const stuck = region(/^Drone stuck: Pin the store clock/);
const ruled = region(/^Failed ruling: Retire sleep calls/);
const clash = region(/^Overlap: Shorten the reconnect wait/);

const jobCallFixes = walk("job-call-fixes", [
  { look: check, say: "A Check failed: which Check, the tests that failed by name, and the end of its output" },
  { look: inside(check, role("list", "Failing tests")), say: "The failing tests, by name" },
  { look: inside(check, text("left: 64s")), say: "The output scrolls from its last lines" },
  { hover: inside(check, button(/^Skip$|^Skip \d/)), say: "Skip says it ends the Check as skipped, not passed" },
  { look: inside(check, button(/^Skip all/)), say: "Skip all is there because more than one Check is unfinished" },
  { look: inside(check, role("textbox", "Message")), say: "Words here go to the Job's Drone" },
  { press: inside(check, button(/^Retry/)), say: "One press on Retry, held while Fleet answers" },
  { look: stuck, say: "A Drone gone quiet: how long, and what it last did" },
  { hover: inside(stuck, role("img", "Quiet for 47 minutes")), say: "The figure names itself on hover" },
  { look: inside(stuck, button(/^Retry step/)), say: "Retry step starts the step again" },
  { type: "Read the failing test first, then pin the clock in the setup\n", into: inside(stuck, role("textbox", "Message")), say: "Words redirect the Drone, and the card leaves" },
  { look: ruled, say: "A ruling that failed: the attempt, what brought it, and the Judge's reason" },
  { press: inside(ruled, button(/^Retry step/)), say: "One press on Retry step" },
  { look: clash, say: "Another Job in the same files: what they share, and the other Job to open" },
  { look: inside(clash, button(/^Open the other Job/)), say: "Open the other Job is the primary here" },
]);

export { jobCallFixes };
