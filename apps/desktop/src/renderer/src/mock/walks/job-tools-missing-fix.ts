// The tools mark as a way out: pressed, it lists what the agent came up without and the act that puts each piece right. Over
// `job-tools-missing`, on the second Job, which lacks its mod and its task tools.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";
import { AGENT } from "../slices/onboarding";

const mark = inside(region(/^Job /), button(/ 2\.1\.312\./));

const terminal = inside(region(/^Job /), role("group", "Terminal", { exact: true }));

const jobToolsMissingFix = walk("job-tools-missing", [
  toSessions,
  { press: deckRow(/Add the billing page/), say: "Open the Job whose agent came up without some tools" },
  { press: inside(region(/ 2\.1\.312\./), button("Dismiss", { exact: true })), say: "Put the band over the message box away" },
  { press: mark, say: "Press the mark in its header. It lists each piece and its act" },
  { press: button("Restart the agent", { exact: true }), say: "Restart the agent. The ring turns, then the row goes" },
  { press: button(`Update ${AGENT.name}`, { exact: true }), say: `Update ${AGENT.name}. The menu closes and the update runs in the Job's terminal` },
  { later: inside(terminal, text(`Updated ${AGENT.name} to 2.1.340`)), say: "The whole output shows. On a clean exit the mark leaves the header" },
]);

export { jobToolsMissingFix as "job-tools-missing-fix" };
