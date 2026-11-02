// A Job's terminals: + then Terminal always opens a new shell, and each tab draws its own, named Terminal 1, Terminal 2 from the ID Fleet gave it.
// A tab keeps its shell when it moves to another pane. Its menu sends the terminal to the agent as a chip in the message box, copies its ID and renames it.
// The Job's agent opens a terminal of its own, which arrives as a tab behind the one in front. Closing a terminal ends its shell, and asks first where a program is running.
// Mock only: nothing is kept. Over the `session-shell` scenario.

import { button, deckRow, inside, region, role, tab, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const thread = region("Pane: Thread", {
  exact: true,
});
const pane = (name: string) =>
  region(`Pane: ${name}`, {
    exact: true,
  });
const named = (name: string) =>
  tab(name, {
    exact: true,
  });
const typed = role("textbox", "Terminal input");
const prompt = text(/slot-1 %/);
const files = text(/CONTRIBUTING\.md/);
const message = role("textbox", "Message");
const item = (name: string) => role("menuitem", name);

const steps: Step[] = [
  toSessions,
  {
    press: deckRow("Deploy the store"),
    say: "Opened",
  },
  {
    press: inside(thread, button("Add tab")),
    say: "+",
  },
  {
    press: item("Terminal"),
    say: "Terminal",
  },
  {
    type: "ls\n",
    into: inside(pane("Terminal 1"), typed),
    say: "ls, Enter",
  },
  {
    look: inside(pane("Terminal 1"), files),
    say: "The worktree's files",
  },
  {
    press: inside(pane("Terminal 1"), button("Add tab")),
    say: "+ again",
  },
  {
    press: item("Terminal"),
    say: "Terminal",
  },
  {
    look: inside(pane("Terminal 2"), prompt),
    say: "A new shell, a fresh prompt",
  },
  {
    press: inside(pane("Terminal 2"), named("Terminal 1")),
    say: "Terminal 1",
  },
  {
    look: inside(pane("Terminal 1"), files),
    say: "Its ls is still there",
  },
  {
    press: inside(pane("Terminal 1"), named("Terminal 2")),
    say: "Terminal 2",
  },
  {
    key: "Meta+d",
    on: inside(pane("Terminal 2"), named("Terminal 2")),
    say: "⌘D splits the pane to the right",
  },
  {
    drag: inside(pane("Terminal 2"), named("Terminal 2")),
    onto: region("Empty pane", {
      exact: true,
    }),
    say: "Terminal 2 into the new pane",
  },
  {
    look: inside(pane("Terminal 1"), files),
    say: "Side by side, each with its own shell",
  },
  {
    context: inside(pane("Terminal 1"), named("Terminal 1")),
    say: "Right click on a tab",
  },
  {
    look: item("Copy terminal ID"),
    say: "Send to agent, Copy terminal ID and Rename sit above the rest",
  },
  {
    press: item("Send to agent"),
    say: "Send to agent",
  },
  {
    look: inside(inside(pane("Thread"), message), text("Terminal 1")),
    say: "A chip in the message box. Nothing is sent",
  },
  {
    context: inside(pane("Terminal 2"), named("Terminal 2")),
    say: "Right click on Terminal 2",
  },
  {
    press: item("Rename"),
    say: "Rename",
  },
  {
    type: "dev\n",
    into: role("textbox", "Rename Terminal 2"),
    say: "A name, Enter",
  },
  {
    look: inside(pane("dev"), named("dev")),
    say: "The tab reads the new name. Its ID is still t2",
  },
  {
    later: inside(pane("dev"), named("dev")),
    say: "Time passes",
  },
  {
    look: inside(pane("dev"), named("Terminal 3")),
    say: "The agent opened Terminal 3 behind the tab in front",
  },
  {
    press: inside(pane("dev"), named("Terminal 3")),
    say: "Terminal 3",
  },
  {
    look: inside(pane("Terminal 3"), text(/VITE v6\.3\.5/)),
    say: "The agent's pnpm dev is running there",
  },
  {
    press: button("Close Terminal 3"),
    say: "Close",
  },
  {
    look: text("A program is running in Terminal 3."),
    say: "The question",
  },
  {
    press: button("End terminal"),
    say: "End terminal",
  },
  {
    press: button("Close dev"),
    say: "Close an idle terminal",
  },
  {
    look: inside(pane("Thread"), named("Terminal 1")),
    say: "No question. Terminal 1 is left",
  },
];

const terminals = walk("session-shell", steps);

export { terminals as "job-terminals" };
