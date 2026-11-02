// A call answered from the Calls HUD: what the Job has done stands beside Now, written by a small model and carried on
// `SessionRecord.so_far`. Over `cockpit-ship-flightdeck`.

import { button, inside, region, role, text, walk } from "../walk";

const CALLS = role("region", "Calls");

export const jobSoFar = walk("cockpit-ship-flightdeck", [
  {
    press: inside(region("Flight controls"), button("Calls")),
    say: "Calls opens on a Job calling",
  },
  {
    look: inside(CALLS, text(/^Read the manifest loader/)),
    say: "What the Job has done so far, beside Now. A small model rewrites it when a turn ends and when a question or a permission comes to you",
  },
  {
    press: inside(CALLS, button(/^Next call/)),
    say: "Next call brings the next Job, with its own summary",
  },
  {
    look: inside(CALLS, text(/^Wrote the changelog script/)),
    say: "At most two sentences in plain words, with no file names, so you can answer from here",
  },
]);
