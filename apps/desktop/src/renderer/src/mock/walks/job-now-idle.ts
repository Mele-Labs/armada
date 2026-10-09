// A Job with nothing to say and no reason given: the panel reads the sentence, and the owner treats it as a bug.

import { inside, region, text, walk } from "../walk";

const now = region("Now");

const w = walk("job-now-idle", [
  { look: inside(now, text("Nothing is actively running on this job")), say: "Nothing to show, and no reason given. That reads as a bug" },
]);

export { w as "job-now-idle" };
