// A procedure that starts from a command: its row reads the item's words, and a press opens the steps with
// the command whole at the top and, inside the block's corner, a key that copies it, as a command with no steps copies on its press.
// The title wraps. A command in a step is set as code and drawn under it with its own key. Over the `needs-you-steps-run` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const TITLE = "Deploy the store Worker once, from this machine";
const BY_STEP = "Deploy the store Worker, a command a step";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const needs = region("Needs you");
  const panel = dialog(TITLE);
  return [
    toSessions,
    { press: inside(sessions, button(/Central store/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, raised from the composer" },
    { look: inside(needs, button(`Open ${TITLE}`)), say: "The row reads the item's words, not its command" },
    { hover: inside(needs, role("img", "Steps")), say: "It wears the Steps glyph" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "A press on the row opens its steps" },
    { look: inside(panel, role("heading", TITLE)), say: "The title wraps, whole" },
    { look: inside(panel, text(/wrangler deploy$/)), say: "The command is at the top, whole" },
    { hover: inside(panel, text(/wrangler deploy$/)), say: "On hover its copy key shows inside the block, at the corner" },
    { press: inside(panel, button("Copy command")), say: "One press copies it" },
    { look: inside(panel, role("checkbox", "Copy the workers.dev address it prints")), say: "The steps follow" },
    { press: inside(panel, role("button", /^Close( panel)?( Esc)?$/)), say: "Closed" },
    { press: inside(needs, button(`Open ${BY_STEP}`)), say: "The same item, with a command in each step" },
    { look: inside(dialog(BY_STEP), text(/pnpm exec wrangler login$/)), say: "Each command is under its step, whole" },
    { hover: inside(dialog(BY_STEP), text(/^pnpm exec wrangler deploy$/)), say: "Each block has its own copy key, in its corner" },
    { press: inside(dialog(BY_STEP), button("Copy step 2 command")), say: "One press copies that step's command" },
    { look: inside(dialog(BY_STEP), role("checkbox", "Copy the workers.dev address it prints")), say: "A step with no command has no key" },
    { look: inside(dialog(BY_STEP), text(/workers\.dev\/health$/)), say: "The command at the top is left out, since step 1 names it" },
  ];
}

const played = walk("needs-you-steps-run", steps());

export { played as "needs-you-steps-run" };
