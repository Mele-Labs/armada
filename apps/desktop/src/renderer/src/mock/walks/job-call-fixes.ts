// A Job that went wrong, fixed from its call: pressing the call in Incoming calls opens the Job with the
// fixes in a strip under its head, so the owner need not open the Job page. A Check failed (Retry, Skip,
// Skip all), a Drone gone quiet (Retry step), a ruling that failed (Retry step) and a Job in another's
// files. The strip carries the acts; the failing tests and the end of the output are on the Job page. Over
// `job-call-fixes`.

import { button, inside, region, role, walk } from "../walk";

const incoming = region("Incoming calls");
const check = role("group", /^Call: Cap the retry backoff/);
const stuck = role("group", /^Call: Pin the store clock/);
const ruled = role("group", /^Call: Retire sleep calls/);
const clash = role("group", /^Call: Shorten the reconnect wait/);

const jobCallFixes = walk("job-call-fixes", [
  { press: inside(incoming, button(/^Check failed: Cap the retry backoff/)), say: "A Check failed: the call opens its Job, with the fixes in a strip under its head" },
  { look: inside(check, button("Retry")), say: "Retry runs the Check again" },
  { hover: inside(check, button("Skip")), say: "Skip says it ends the Check as skipped, not passed" },
  { look: inside(check, button("Skip all")), say: "Skip all is there because more than one Check is unfinished" },
  { press: inside(check, button("Retry")), say: "One press on Retry, held while Fleet answers" },
  { press: inside(incoming, button(/^Drone stuck: Pin the store clock/)), say: "A Drone gone quiet: the call opens its Job" },
  { look: inside(stuck, button("Retry step")), say: "Retry step starts the step again" },
  { press: inside(incoming, button(/^Failed ruling: Retire sleep calls/)), say: "A ruling that failed: the call opens its Job" },
  { look: inside(ruled, button("Retry step")), say: "Retry step is the one fix" },
  { press: inside(ruled, button("Retry step")), say: "One press on Retry step" },
  { press: inside(incoming, button(/^Overlap: Shorten the reconnect wait/)), say: "Another Job in the same files: the call opens its Job" },
  { look: clash, say: "The strip names what they share" },
  { look: inside(clash, button(/Open the other Job/)), say: "Open the other Job is the primary here" },
]);

export { jobCallFixes };
