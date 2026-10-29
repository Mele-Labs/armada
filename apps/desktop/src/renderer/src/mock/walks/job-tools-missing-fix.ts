// The tools mark as a way out: pressed, it lists what the agent came up without and the act that puts each piece right. Over
// `job-tools-missing`, on the second Job, which lacks its mod and its task tools.

import { button, deckRow, inside, region, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const mark = inside(region(/^Job /), button(/^Claude Code 2\.1\.312\./));

const jobToolsMissingFix = walk("job-tools-missing", [
  toSessions,
  { press: deckRow(/Add the billing page/), say: "Open the Job whose agent came up without some tools" },
  { press: mark, say: "Press the mark in its header. It lists each piece and its act" },
  { press: button("Restart the agent", { exact: true }), say: "Restart the agent. The ring turns, then the row goes" },
  { press: button("Update Claude Code", { exact: true }), say: "Update Claude Code. The CLI refuses, and its own words stay under the row" },
  { press: button("Update Claude Code", { exact: true }), say: "Press it again. The update goes through and the mark goes with the last row" },
]);

export { jobToolsMissingFix as "job-tools-missing-fix" };
