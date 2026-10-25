// A Job that failed on an expired Claude sign-in waits on a command, Sign in to Claude. Run opens the
// sign-in in the Job's terminal. Over the `sign-in-expired` scenario.

import { button, deckRow, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const thread = region("Thread");
const run = inside(needs, role("group", /claude auth login/));
const pane = role("group", "Terminal", { exact: true });

const steps: Step[] = [
  toSessions,
  { look: deckRow(/Add the billing page/), say: "A Job needs him" },
  { press: deckRow(/Add the billing page/), say: "Opened" },
  { look: inside(thread, text(/Failed to authenticate: OAuth session expired/)), say: "The failed turn" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you" },
  { look: inside(run, button("Run", { exact: true })), say: "Sign in to Claude" },
  { press: inside(run, button("Run", { exact: true })), say: "Run" },
  { later: inside(pane, text(/Login successful/)), say: "Signed in" },
];

const signInExpired = walk("sign-in-expired", steps);

export { signInExpired as "sign-in-expired" };
