// The band over a Job's message box when its agent came up without some tools, and the pulsing mark that stands in once it is put away.
// Over `job-tools-missing`, on the second Job.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";
import { AGENT } from "../slices/onboarding";

const band = region(/ 2\.1\.312\./);

const job = region(/^Job /);

const terminal = inside(job, role("group", "Terminal", { exact: true }));

const jobToolsMissingBand = walk("job-tools-missing", [
  toSessions,
  { press: deckRow(/Add the billing page/), say: "Open the Job whose agent came up without some tools. The band shows over its message box, and the header's mark does not pulse" },
  { press: inside(band, button("Dismiss", { exact: true })), say: "Dismiss the band. It stays away until something else is missing" },
  { press: inside(job, button(/ 2\.1\.312\./)), say: "The header's mark pulses now. Press it for the menu" },
  { press: button("Restart the agent", { exact: true }), say: "Restart the agent. Its line goes" },
  { press: button(`Update ${AGENT.name}`, { exact: true }), say: `Update ${AGENT.name}. It runs in the Job's terminal` },
  { later: inside(terminal, text(`Updated ${AGENT.name} to 2.1.340`)), say: "On a clean exit the mark goes" },
]);

export { jobToolsMissingBand as "job-tools-missing-band" };
