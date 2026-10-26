// A command a Job waits on keeps running when the Job is left: a login waiting on the browser is still going when
// the owner comes back, with what it printed. Three other runs end three ways, and each mark says which: an exit
// code, Stop, and a signal. Over `run-outlives-its-card`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const pane = role("group", "Terminal", { exact: true });
const run = (command: string) => inside(needs, role("group", command));
const mark = inside(pane, role("img", /^(Running|Failed|Stopped|Passed)$/));

const runOutlivesItsCard = walk("run-outlives-its-card", [
  toSessions,
  { press: deckRow(/Sign in to the deploy host/), say: "A Job waiting on four commands" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
  { press: inside(run("deployctl login --web"), button("Run")), say: "Run the login: it opens in the docked terminal, as it does against a real Fleet" },
  { look: inside(pane, role("img", "Running")), say: "It waits on the browser" },
  { press: deckRow(/Release notes script/), say: "Leave the Job" },
  { press: deckRow(/Sign in to the deploy host/), say: "And come back" },
  { look: inside(pane, role("img", "Running")), say: "The login is still running, in its pane" },
  { look: inside(pane, text("Waiting for you to finish in the browser")), say: "With what it printed" },
  { press: inside(run("pnpm lint --strict"), button("Run")), say: "Run the linter, which takes the pane's place" },
  { hover: inside(pane, role("img", "Failed")), say: "Its mark says Exit 1, the exit code" },
  { press: inside(run("pnpm watch-build"), button("Run")), say: "Run the build watcher" },
  { press: inside(pane, button("Stop")), say: "Stop it" },
  { hover: inside(pane, role("img", "Stopped")), say: "Its mark says Stopped: you stopped it" },
  { press: inside(run("./scripts/render-docs"), button("Run")), say: "Run the docs render" },
  { hover: mark, say: "Its mark says Ended by a signal: no exit code, and nobody pressed Stop" },
]);

export { runOutlivesItsCard as "run-outlives-its-card" };
