// A command a Job waits on keeps running when the Job is left: a login waiting on the browser is still going when
// the owner comes back, with what it printed. Three other runs end three ways, and each mark says which: an exit
// code, Stop, and a signal. Over `run-outlives-its-card`.

import { button, deckRow, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const run = (command: string) => inside(needs, role("group", command));
const mark = (command: string) => inside(run(command), role("img", /^(Failed|Stopped|Passed)$/));
const output = (command: string) => inside(run(command), role("log", "Output"));

const runOutlivesItsCard = walk("run-outlives-its-card", [
  toSessions,
  { press: deckRow(/Sign in to GitHub/), say: "A Job waiting on four commands" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
  { press: inside(run("gh auth login --web"), button("Run")), say: "Run the login" },
  { look: inside(run("gh auth login --web"), button("Stop")), say: "It waits on the browser, and Stop is its key" },
  { press: deckRow(/Release notes script/), say: "Leave the Job" },
  { press: deckRow(/Sign in to GitHub/), say: "And come back" },
  { look: inside(run("gh auth login --web"), button("Stop")), say: "The login is still running" },
  { look: output("gh auth login --web"), say: "With what it printed" },
  { press: inside(run("pnpm vitest run"), button("Run")), say: "Run the tests" },
  { hover: mark("pnpm vitest run"), say: "Its mark says Exit 1, the exit code" },
  { press: inside(run("tail -f build.log"), button("Run")), say: "Run the build log" },
  { press: inside(run("tail -f build.log"), button("Stop")), say: "Stop it" },
  { hover: mark("tail -f build.log"), say: "Its mark says Stopped: you stopped it" },
  { press: inside(run("./scripts/render-docs"), button("Run")), say: "Run the docs render" },
  { hover: mark("./scripts/render-docs"), say: "Its mark says Ended by a signal: no exit code, and nobody pressed Stop" },
]);

export { runOutlivesItsCard as "run-outlives-its-card" };
