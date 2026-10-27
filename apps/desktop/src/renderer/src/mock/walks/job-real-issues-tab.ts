// Now on a real Job. Job c2d23c4c, an Issues tab for Bridge, at call 396 of 676: the plan, the marks and the last sixty calls are its own. Now leads with what its tool
// calls show it doing; the marks sit under their groups, each in progress with how long since the agent changed it, and an old one dims. Over `job-real-issues-tab`.

import { deckRow, inside, region, text, walk } from "../walk";

const now = region("Now", { exact: true });
const next = region("Next", { exact: true });

export const jobRealIssuesTab = walk("job-real-issues-tab", [
  { press: deckRow("Issues tab beside Reviews"), say: "Open the Job. Its plan and its marks are the real ones from call 396" },
  { look: inside(now, text("Running multiple commands")), say: "Now leads with what its tool calls show it doing, read from the thread and not from the task marks" },
  { look: inside(now, text("Tests against a fake MCP server")), say: "A mark changed a moment ago reads normally, with the time since at the right" },
  { hover: inside(now, text(/^\d+ h/)), say: "A mark nobody has changed for a long while dims. Hover the time: since the agent last updated this task" },
  { look: inside(now, text("Full key map for the tab")), say: "Several marks in progress all show, each with its own time, the oldest dimmed" },
  { look: next, say: "The group not begun waits in Next" },
]);
