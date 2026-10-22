// A Job whose own turn ended while a subagent it dispatched keeps building and running Checks. The Cockpit files it
// under Working and says a subagent is working; the Checks tile and layer say the agent that ran each Check gets
// its result. Over the `session-checks` scenario.

import { button, deckRow, inside, place, region, role, text, walk } from "../walk";

const JOBS = region("Jobs", { exact: true });
const CONTROLS = region("Flight controls");
const CHECKS = region("Checks", { exact: true });

const sessionChecksWhileASubagentWorks = walk("session-checks", [
  { press: place("Cockpit"), say: "Cockpit" },
  { look: inside(JOBS, role("img", "Subagent working")), say: "The Job whose own turn ended is under Working, and its row says a subagent is working" },
  { press: deckRow("Session check rows"), say: "Open the Job" },
  { look: inside(CONTROLS, role("img", "1 subagents running")), say: "Subagents counts the one that is running" },
  { hover: inside(CONTROLS, button("Checks")), say: "Checks tip: a Check the agent ran is still out. The agent gets its result when it ends" },
  { press: inside(CONTROLS, button("Checks")), say: "Open the Checks layer" },
  { look: inside(CHECKS, text("The agent that ran each Check gets its result when the run ends.")), say: "The agent that ran each Check gets its result when the run ends" },
  { later: inside(CHECKS, button("app_smoke, running")), say: "The running Check ends" },
  { look: inside(CHECKS, button("app_smoke, failed")), say: "It failed, and the agent has its result" },
]);

export { sessionChecksWhileASubagentWorks as "session-checks-while-a-subagent-works" };
