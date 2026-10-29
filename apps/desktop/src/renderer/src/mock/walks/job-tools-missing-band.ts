// The band take on the tools mark: a band over the Job's message box that says what the agent came up without and carries each act,
// until nothing is missing. Over `job-tools-missing`, on the second Job, with `?take=band`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const band = region(/^Claude Code 2\.1\.312\./);

const terminal = inside(region(/^Job /), role("group", "Terminal", { exact: true }));

const jobToolsMissingBand = walk("job-tools-missing", [
  toSessions,
  { press: deckRow(/Add the billing page/), say: "Open the Job whose agent came up without some tools" },
  { look: band, say: "The band over the message box: the CLI version, each piece missing and its act" },
  { press: inside(band, button("Restart the agent", { exact: true })), say: "Restart the agent. Its line goes from the band" },
  { press: inside(band, button("Update Claude Code", { exact: true })), say: "Update Claude Code. It runs in the Job's terminal" },
  { later: inside(terminal, text(/Updated Claude Code to 2\.1\.340/)), say: "On a clean exit the band goes" },
]);

export { jobToolsMissingBand as "job-tools-missing-band" };
