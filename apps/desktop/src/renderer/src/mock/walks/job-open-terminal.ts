// A terminal the Job's agent opened while the person was away has no tab: + lists it under Terminal, and picking it opens it in a tab.
// Mock only: nothing is kept. Over the `session-shell-away` scenario.

import { button, deckRow, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const here = region(/^Pane: (Thread|Terminal)/);

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Deploy the store"),
    say: "Opened",
  },
  {
    later: inside(here, tab(/^Thread(: |$)/)),
    say: "Time passes: the agent opens a terminal. The Job has no tab for it",
  },
  {
    press: inside(here, button("Add tab")),
    say: "+",
  },
  {
    look: role("menuitem", "Terminal", {
      exact: true,
    }),
    say: "Terminal opens a new shell, as before",
  },
  {
    look: role("menuitem", "Open Terminal 1", {
      exact: true,
    }),
    say: "The agent's terminal is listed under it",
  },
  {
    press: role("menuitem", "Open Terminal 1", {
      exact: true,
    }),
    say: "Open Terminal 1",
  },
  {
    look: inside(
      here,
      tab("Terminal 1", {
        exact: true,
      }),
    ),
    say: "The agent's terminal, in a tab of the pane",
  },
];

const openTerminal = walk("session-shell-away", steps);

export { openTerminal as "job-open-terminal" };
