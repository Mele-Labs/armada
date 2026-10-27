// A command in Needs you that asks for a passphrase: Run opens a terminal under it, the passphrase is typed
// there and is not shown, and the program exits 0. Over the `needs-you-run-prompts` scenario.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const needs = region("Needs you");
const thread = region("Thread");
const run = inside(needs, role("group", "./scripts/unlock-signing-key"));
const pane = role("group", "Terminal", { exact: true });
const typed = role("textbox", "Terminal input");

const steps: Step[] = [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(/Sign the release tag/)), say: "Opened" },
  { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened" },
  { look: inside(run, button("Run", { exact: true })), say: "The command with Run" },
  { press: inside(run, button("Run", { exact: true })), say: "Run" },
  { look: inside(pane, text(/Passphrase:/)), say: "The program asks for a passphrase" },
  { look: inside(pane, typed), say: "The terminal under the command has the keys" },
  { type: "correct horse\n", into: typed, say: "A passphrase, Enter. The terminal does not show it" },
  { look: inside(pane, text("Key unlocked for 8 hours")), say: "The key is unlocked" },
  { look: inside(pane, role("img", "Passed")), say: "Exit 0" },
  { look: inside(thread, button(/^Exit 0/)), say: "The Job was sent the exit" },
];

const prompts = walk("needs-you-run-prompts", steps);

export { prompts as "needs-you-run-prompts" };
