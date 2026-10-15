// A procedure that starts from a command: its row reads the item's words, and a press opens the steps with
// the command whole at the top and a key that copies it, as a command with no steps copies on its press.
// The title wraps. Over the `needs-you-steps-run` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const TITLE = "Deploy the store Worker once, from this machine";

function steps(): Step[] {
  const sessions = region("Sessions");
  const needs = region("Needs you");
  const panel = dialog(TITLE);
  return [
    toSessions,
    { press: inside(sessions, button(/Central store/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened at the foot of the thread" },
    { look: inside(needs, button(`Open ${TITLE}`)), say: "The row reads the item's words, not its command" },
    { hover: inside(needs, role("img", "Steps")), say: "It wears the Steps glyph" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "A press opens its steps" },
    { look: inside(panel, role("heading", TITLE)), say: "The title wraps, whole" },
    { look: inside(panel, text(/wrangler deploy$/)), say: "The command is at the top, whole" },
    { press: inside(panel, button("Copy command")), say: "One press copies it" },
    { look: inside(panel, role("checkbox", "Copy the workers.dev address it prints")), say: "The steps follow" },
  ];
}

const played = walk("needs-you-steps-run", steps());

export { played as "needs-you-steps-run" };
