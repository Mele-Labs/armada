// The Job's shell in a window of its own: Move to window on the Terminal tab's menu moves the shell there with all it printed,
// the tab says where it is, Move to the pane or Dock on the window's head brings the shell back, and the window's own close
// puts the shell back in its tab with it still running.
// The mock draws the window over the page, as it draws the walk window. A walk finds what is on screen and cannot find what
// is not, so a window being gone is shown by the pane's text coming back and held by `job-shell.test.tsx`.
// Over the `session-shell` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const here = region(/^Pane: (Thread|zsh)/);
const pane = inside(
  here,
  role("group", "Terminal", {
    exact: true,
  }),
);
const typed = role("textbox", "Terminal input");
const popOut = role("dialog", "Terminal: Deploy the store", {
  exact: true,
});
const inWindow = inside(
  popOut,
  role("group", "Terminal", {
    exact: true,
  }),
);
const files = text(/CONTRIBUTING\.md/);
const where = text(/\.armada\/slots\/slot-1/);

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
    say: "The shell, in a tab of the pane",
  },
  {
    type: "ls\n",
    into: typed,
    say: "ls, Enter",
  },
  {
    look: inside(pane, files),
    say: "The worktree's files",
  },
  {
    context: inside(here, tab(/^zsh/)),
    say: "The tab's menu",
  },
  {
    press: role("menuitem", "Move to window"),
    say: "Move to window",
  },
  {
    look: inside(inWindow, files),
    say: "The window shows everything the shell printed",
  },
  {
    look: inside(here, button("Move to the pane")),
    say: "The pane keeps its tab and says where the shell is",
  },
  {
    type: "pwd\n",
    into: inside(popOut, typed),
    say: "pwd, Enter, in the window",
  },
  {
    look: inside(inWindow, where),
    say: "The same shell answers in its window",
  },
  {
    press: inside(here, button("Move to the pane")),
    say: "Move to the pane",
  },
  {
    look: inside(pane, where),
    say: "Back in the tab with everything it printed",
  },
  {
    context: inside(here, tab(/^zsh/)),
    say: "The tab's menu",
  },
  {
    press: role("menuitem", "Move to window"),
    say: "Move to window again",
  },
  {
    press: inside(
      popOut,
      button("Dock", {
        exact: true,
      }),
    ),
    say: "Dock",
  },
  {
    look: inside(pane, files),
    say: "Docked again with the files it printed",
  },
  {
    look: inside(pane, where),
    say: "The pwd output too",
  },
];

const shellWindow = walk("session-shell", steps);

export { shellWindow as "job-shell-window" };
