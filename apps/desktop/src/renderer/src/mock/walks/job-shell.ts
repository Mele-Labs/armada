// The Job's shell: one terminal a Job keeps open in its worktree, which the person types into and the
// agent types into through its tools. Terminal in Flight control opens it, and is lit while it shows.
// In the Cockpit it docks in a panel of its own between the Job and the merge line, so the thread keeps
// the Job's whole pane, and a block Run docks in the Job.
// A toggle on its head moves it to the side panel and back, and saves where it opens, as Settings, Terminal does.
// Close puts the side panel away with the shell still running.
// Over the `session-shell` scenario.

import { button, inside, place, region, role, systemPage, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const job = region(/^Job: /);
const thread = inside(job, region("Thread"));
const pane = role("group", "Terminal", { exact: true });
// The Cockpit's panel for the shell, between the Job and the merge queue.
const docked = inside(region("Terminal", { exact: true }), pane);
const run = inside(job, pane);
const typed = role("textbox", "Terminal input");
const terminal = inside(region("Flight controls"), button("Terminal", { exact: true }));
const block = inside(thread, role("group", /wrangler deploy/));
const opensIn = role("combobox", "Open a Job's terminal in", { exact: true });
const panel = role("dialog", "Terminal", { exact: true });
const served = text(/Local:.*localhost:5173/);
// The server's newest line, which shows at any height the panel is dragged to.
const printing = text(/hmr update/);

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Deploy the store/)), say: "Opened" },
  { look: terminal, say: "Terminal, in Flight control" },
  { press: terminal, say: "Terminal" },
  { look: inside(docked, text(/slot-1 %/)), say: "A shell in the Job's worktree, in its own panel between the Job and the merge line" },
  { look: inside(job, role("textbox", "Message")), say: "Nothing docks under the thread. It keeps its whole height" },
  { type: "ls\n", into: typed, say: "ls, Enter" },
  { later: inside(docked, text(/CONTRIBUTING\.md/)), say: "The worktree's files" },
  { look: inside(thread, text("Starting the dev server in the terminal.")), say: "The agent starts the dev server" },
  { look: inside(docked, text(/VITE v6\.3\.5/)), say: "It typed pnpm dev into the same shell" },
  { look: inside(docked, printing), say: "The server's output streams in" },
  { key: "Control+c", on: typed, say: "Ctrl-C" },
  { look: inside(docked, text(/\^C/)), say: "The server stopped and the prompt is back" },
  { press: inside(docked, button("Hide")), say: "Hidden. The shell keeps running" },
  { press: terminal, say: "Terminal" },
  { look: inside(docked, text(/\^C/)), say: "The same shell, as it was left" },
  { press: inside(block, button("Run", { exact: true })), say: "Run on the deploy command" },
  { look: inside(run, button("Stop")), say: "The run docks in the Job, above its box" },
  { look: inside(docked, button("End shell")), say: "The shell stays in its own panel" },
  { press: inside(docked, button("End shell")), say: "Ended the shell" },
  { look: inside(docked, text("Shell ended")), say: "Shell ended" },
  { press: inside(docked, button("Close")), say: "Closed. The run stays" },
  { later: inside(run, button("Stop")), say: "The run, in the Job" },
  { look: inside(docked, button("End shell")), say: "The agent opened a new shell, and it shows in its panel" },
  { look: inside(docked, served), say: "Its dev server, in the new shell" },
  { press: inside(run, button("Stop")), say: "Stop the run" },
  { press: inside(run, button("Close")), say: "Closed the run. The shell stays docked" },
  { press: inside(docked, button("Move to the side panel")), say: "Move to the side panel" },
  { look: inside(panel, served), say: "The same shell in the side panel, its output kept" },
  { press: inside(panel, button(/^Close/)), say: "Close. The shell keeps running" },
  { press: place("System"), say: "System" },
  { press: systemPage("Terminal"), say: "Terminal settings" },
  { look: opensIn, say: "The toggle saved it: side panel" },
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Deploy the store/)), say: "Back to the Job" },
  { press: terminal, say: "Terminal" },
  { look: inside(panel, served), say: "In the side panel, the thread at full height" },
  { press: inside(panel, button("Dock", { exact: true })), say: "Dock" },
  { look: inside(docked, printing), say: "Docked again in its panel, the server still printing" },
  { press: terminal, say: "Terminal again. Hidden, and the shell keeps running" },
];

const shell = walk("session-shell", steps);

export { shell as "job-shell" };
