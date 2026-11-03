// A Job's agent opens a terminal while the person is away from the Job: Fleet has it, the Job's pane has no tab for it. The + lists it.
// Over the `session-shell` Job, whose agent opens its terminal where the window hears it open.

import { agentOpensTerminalAway, shellScenario } from "./session-shell";

export const s323SessionShellAway = shellScenario(
  "session-shell-away",
  "A terminal the Job's agent opened while the person was away, with no tab until the + opens it",
  agentOpensTerminalAway,
);
