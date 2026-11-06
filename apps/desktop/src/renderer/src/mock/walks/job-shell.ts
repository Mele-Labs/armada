// A terminal in a Job: + then Terminal opens a new shell in a tab and the pane tells it its size, ⌥⇧0 brings the Terminal tab forward or opens one,
// and closing a Terminal tab ends its shell. The Job's agent types into a terminal of its own, which arrives as a tab behind. A block Run docks in the Job.
// Several terminals side by side are the walk `job-terminals`; a terminal in a window of its own is the walk `job-shell-window`.
// Over the `session-shell` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = inside(job, region(/^Pane: Thread/));
const here = region(/^Pane: (Thread|Terminal)/);
const shellIn = (name: string) =>
  inside(
    region(`Pane: ${name}`, {
      exact: true,
    }),
    role("group", "Terminal", {
      exact: true,
    }),
  );
const pane = shellIn("Terminal 1");
const agents = shellIn("Terminal 2");
const typed = role("textbox", "Terminal input");
const block = inside(thread, role("group", /wrangler deploy/));
// The server's newest line, which shows at any height the pane is dragged to.
const printing = text(/hmr update/);

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Deploy the store"),
    say: "Opened",
  },
  {
    press: inside(here, button("Add tab")),
    say: "+",
  },
  {
    press: role("menuitem", "Terminal"),
    say: "Terminal",
  },
  {
    look: inside(pane, text(/slot-1 %/)),
    say: "A shell in the Job's worktree, in a tab of the pane",
  },
  {
    type: "ls\n",
    into: typed,
    say: "ls, Enter",
  },
  {
    later: inside(pane, text(/CONTRIBUTING\.md/)),
    say: "The worktree's files",
  },
  {
    look: inside(
      here,
      tab("Terminal 2", {
        exact: true,
      }),
    ),
    say: "The Job's agent opened a terminal of its own, behind this one",
  },
  {
    press: inside(
      here,
      tab("Terminal 2", {
        exact: true,
      }),
    ),
    say: "Terminal 2",
  },
  {
    look: inside(agents, text(/VITE v6\.3\.5/)),
    say: "The agent typed pnpm dev into it",
  },
  {
    look: inside(agents, printing),
    say: "The server's output streams in",
  },
  {
    key: "Control+c",
    on: inside(agents, typed),
    say: "Ctrl-C",
  },
  {
    look: inside(agents, text(/\^C/)),
    say: "The server stopped and the prompt is back",
  },
  {
    press: inside(agents, button("End shell")),
    say: "Ended the shell",
  },
  {
    look: inside(agents, text("Shell ended")),
    say: "Shell ended",
  },
  {
    press: inside(agents, button("New shell")),
    say: "New shell",
  },
  {
    look: inside(agents, button("End shell")),
    say: "A new shell, in the same tab",
  },
  {
    press: button("Close Terminal 2"),
    say: "Closed the tab. Its shell ends",
  },
  {
    press: button("Close Terminal 1"),
    say: "Closed the other one too",
  },
  {
    key: "Alt+Shift+0",
    on: tab(/^Thread(: |$)/),
    say: "⌥⇧0 opens a Terminal tab where the Job has none",
  },
  {
    look: inside(shellIn("Terminal 1"), text(/slot-1 %/)),
    say: "A new shell, with nothing from the one that was closed",
  },
  {
    press: inside(
      block,
      button("Run", {
        exact: true,
      }),
    ),
    say: "Run on the deploy command",
  },
  {
    look: inside(job, button("Stop")),
    say: "The run docks in the Job, and the terminal stays in its tab",
  },
];

const shell = walk("session-shell", steps);

export { shell as "job-shell" };
