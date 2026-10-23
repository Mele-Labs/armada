// The Job's shell: one terminal a Job keeps open in its worktree, docked where a block's run docks, which
// the person types into and the agent types into through its tools. Beside a block Run the two are tabs.
// Over the `session-shell` scenario.

import { button, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = region("Thread");
const pane = role("group", "Terminal", { exact: true });
const typed = role("textbox", "Terminal input");
const terminal = inside(job, button("Terminal", { exact: true }));
const runTab = inside(pane, tab(/^cd ~\/Development.*…$/));
const block = inside(thread, role("group", /wrangler deploy/));

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Deploy the store/)), say: "Opened" },
  { look: terminal, say: "Terminal, in the Job's header" },
  { press: terminal, say: "Terminal" },
  { look: inside(pane, text(/slot-1 %/)), say: "A shell in the Job's worktree, docked above the box" },
  { type: "ls\n", into: typed, say: "ls, Enter" },
  { later: inside(pane, text(/CLAUDE\.md/)), say: "The worktree's files" },
  { look: inside(thread, text("Starting the dev server in the terminal.")), say: "The agent starts the dev server" },
  { look: inside(pane, text(/VITE v6\.3\.5/)), say: "It typed pnpm dev into the same shell" },
  { look: inside(pane, text(/hmr update/)), say: "The server's output streams in" },
  { key: "Control+c", on: typed, say: "Ctrl-C" },
  { look: inside(pane, text(/\^C/)), say: "The server stopped and the prompt is back" },
  { press: inside(pane, button("Hide")), say: "Hide. The shell keeps running" },
  { press: terminal, say: "Terminal" },
  { look: inside(pane, text(/\^C/)), say: "The same shell, as it was left" },
  { press: inside(block, button("Run", { exact: true })), say: "Run on the deploy command" },
  { look: runTab, say: "The run and the shell, side by side as tabs" },
  { hover: runTab, say: "The whole command, on hover" },
  { press: inside(pane, tab("Terminal")), say: "Back to the shell" },
  { press: inside(pane, button("End shell")), say: "End shell" },
  { look: inside(pane, text("Shell ended")), say: "Ended, with New shell and Close" },
  { press: inside(pane, button("Close")), say: "Closed. The run stays" },
  { later: inside(pane, button("Stop")), say: "The run, alone" },
  { look: inside(pane, tab("Terminal")), say: "The agent opened a new shell, and it shows by itself" },
  { look: inside(pane, text(/Local:.*localhost:5173/)), say: "Its dev server, in the new shell" },
];

const shell = walk("session-shell", steps);

export { shell as "job-shell" };
