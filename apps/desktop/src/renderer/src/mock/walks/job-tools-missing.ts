// A Job whose agent came up without some of its Armada tools: the mark on its row in the Jobs list, and the same mark in its
// header. Over `job-tools-missing`.

import { deckRow, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const mark = role("img", /Claude Code 2\.1\.312\. No armada mod\. No task tools\./);

const jobToolsMissing = walk("job-tools-missing", [
  toSessions,
  { hover: inside(region("Jobs", { exact: true }), mark), say: "The mark on the second Job's row. Its card's Tools line names the CLI version and what is missing" },
  { press: deckRow(/Add the billing page/), say: "Open that Job" },
  { look: inside(region(/^Job /), mark), say: "The same mark in its header" },
]);

export { jobToolsMissing as "job-tools-missing" };
