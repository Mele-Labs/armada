// The Job's shell: one terminal a Job keeps open in its worktree, which the person types into and the agent types into
// through its tools. It is a Terminal tab like any other: + opens it at the pane's size, ⌥⇧0 brings it forward or opens one,
// and closing the last Terminal tab puts it away with the shell still running. A block Run docks in the Job.
// Its window of its own is the walk `job-shell-window`.
// Over the `session-shell` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = inside(job, region(/^Pane: Thread/));
const here = region(/^Pane: (Thread|zsh)/);
const pane = inside(
  here,
  role("group", "Terminal", {
    exact: true,
  }),
);
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
    look: inside(pane, text(/VITE v6\.3\.5/)),
    say: "The agent typed pnpm dev into the same shell",
  },
  {
    look: inside(pane, printing),
    say: "The server's output streams in",
  },
  {
    key: "Control+c",
    on: typed,
    say: "Ctrl-C",
  },
  {
    look: inside(pane, text(/\^C/)),
    say: "The server stopped and the prompt is back",
  },
  {
    press: button(/^Close zsh/),
    say: "Closed the tab. The shell keeps running",
  },
  {
    key: "Alt+Shift+0",
    on: tab("Thread", {
      exact: true,
    }),
    say: "⌥⇧0 opens a Terminal tab where the Job has none",
  },
  {
    look: inside(pane, text(/\^C/)),
    say: "The same shell, as it was left",
  },
  {
    press: inside(pane, button("End shell")),
    say: "Ended the shell",
  },
  {
    look: inside(pane, text("Shell ended")),
    say: "Shell ended",
  },
  {
    press: inside(pane, button("New shell")),
    say: "New shell",
  },
  {
    look: inside(pane, button("End shell")),
    say: "A new shell, in the same tab",
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
    say: "The run docks in the Job, and the shell stays in its tab",
  },
];

const shell = walk("session-shell", steps);

export { shell as "job-shell" };
