// A terminal the Job's agent shows comes to the front in that Job's panes, while the person is on another Job and while they are on it.
// The person is not moved to the Job. Mock only: nothing is kept. Over the `session-shell-shown` scenario.

import { button, deckRow, inside, region, role, tab, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region(/^Pane: Thread/);
const inFront = (name: string) =>
  region(`Pane: ${name}`, {
    exact: true,
  });

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Release notes script"),
    say: "Release notes script",
  },
  {
    later: thread,
    say: "Time passes. The agent of Deploy the store opens Terminal 1 and Terminal 2, and shows Terminal 2",
  },
  {
    look: thread,
    say: "Release notes script is still open",
  },
  {
    press: deckRow("Deploy the store"),
    say: "Deploy the store",
  },
  {
    look: inFront("Terminal 2"),
    say: "Terminal 2 is in front",
  },
  {
    press: inside(inFront("Terminal 2"), button("Add tab")),
    say: "+",
  },
  {
    press: role("menuitem", "Open Terminal 1", {
      exact: true,
    }),
    say: "Open Terminal 1",
  },
  {
    look: inFront("Terminal 1"),
    say: "Terminal 1 is in front. Terminal 2 is behind it",
  },
  {
    press: inside(
      thread,
      tab("Thread", {
        exact: true,
      }),
    ),
    say: "Thread",
  },
  {
    later: thread,
    say: "Time passes. The agent shows Terminal 2",
  },
  {
    look: inFront("Terminal 2"),
    say: "Terminal 2 is in front. The Thread pane keeps the focus",
  },
];

const showsTerminal = walk("session-shell-shown", steps);

export { showsTerminal as "job-shows-terminal" };
