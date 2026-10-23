// The Job's shell in a window of its own: Open in a window on the docked pane moves the shell there with all it printed,
// Terminal while the window is open brings it forward, the window's own close puts the shell away with it still running,
// Terminal opens the window again, and Dock on its head brings the shell back into Bridge and saves where it opens.
// The mock draws the window over the page, as it draws the walk window. A walk finds what is on screen and cannot find
// what is not, or read a button's pressed state, so a window or pane being gone and the tile being lit are shown by the
// tile's tooltip in a hover and held by `job-shell.test.tsx`.
// Over the `session-shell` scenario.

import { button, inside, place, region, role, systemPage, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const pane = role("group", "Terminal", { exact: true });
// The Cockpit's panel for the shell, between the Job and the merge queue.
const docked = inside(region("Terminal", { exact: true }), pane);
const typed = role("textbox", "Terminal input");
const terminal = inside(region("Flight controls"), button("Terminal", { exact: true }));
const popOut = role("dialog", "Terminal: Deploy the store", { exact: true });
const inWindow = inside(popOut, pane);
const files = text(/CONTRIBUTING\.md/);
const here = text(/\.armada\/slots\/slot-1/);
const opensIn = role("combobox", "Open a Job's terminal in", { exact: true });

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Deploy the store/)), say: "Opened" },
  { press: terminal, say: "Terminal" },
  { look: inside(docked, text(/slot-1 %/)), say: "The shell, docked between the Job and the merge line" },
  { type: "ls\n", into: typed, say: "ls, Enter" },
  { look: inside(docked, files), say: "The worktree's files" },
  { press: inside(docked, button("Open in a window")), say: "Open in a window" },
  { look: inside(inWindow, files), say: "The window shows everything the shell printed" },
  { hover: terminal, say: "Terminal is still lit, and brings the window to the front" },
  { type: "pwd\n", into: inside(popOut, typed), say: "pwd, Enter, in the window" },
  { look: inside(inWindow, here), say: "The same shell answers in its window" },
  { press: terminal, say: "Terminal, with the window open" },
  { look: inside(popOut, button("Close window")), say: "The window is still open, brought forward" },
  { press: inside(popOut, button("Close window")), say: "Close window" },
  { hover: terminal, say: "Terminal is unlit and says Open the Job's terminal. The shell keeps running" },
  { press: terminal, say: "Terminal" },
  { look: inside(inWindow, files), say: "The window opens again with the files" },
  { look: inside(inWindow, here), say: "And the pwd output, as the shell printed it" },
  { press: inside(popOut, button("Dock", { exact: true })), say: "Dock" },
  { look: inside(docked, files), say: "Docked again with everything it printed" },
  { look: inside(docked, here), say: "The pwd output too" },
  { press: place("System"), say: "System" },
  { press: systemPage("Terminal"), say: "Terminal settings" },
  { look: inside(opensIn, text("docked")), say: "Dock saved where it opens: docked" },
];

const shellWindow = walk("session-shell", steps);

export { shellWindow as "job-shell-window" };
