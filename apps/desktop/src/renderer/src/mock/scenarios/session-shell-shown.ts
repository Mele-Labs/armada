// A Job's agent shows one of its terminals. The first time passes, it opens t1 and t2 while the person is away, neither with a tab, and shows t2;
// each time after, it shows t2 again. Over the `session-shell` Job. The walk `job-shows-terminal` plays it.

import { agentOpensTerminalAway, agentShowsTerminal, shellScenario } from "./session-shell";

export const s345SessionShellShown = shellScenario(
  "session-shell-shown",
  "A Job's agent shows one of its terminals, whose tab comes to the front in that Job without moving the person",
  (sessionId) => {
    if (agentShowsTerminal(sessionId, "t2")) return;
    agentOpensTerminalAway(sessionId);
    agentOpensTerminalAway(sessionId);
    agentShowsTerminal(sessionId, "t2");
  },
);
