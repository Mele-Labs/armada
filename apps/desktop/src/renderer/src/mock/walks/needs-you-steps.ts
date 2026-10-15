// An item that waits on the person for a procedure: its row wears the Steps glyph, and a press opens the
// steps beside the thread in place of the link. The link is the key at the top, each step has a tick that
// is kept, and the reply box under them answers the item. Over the `needs-you-steps` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const TITLE = "Register the GitHub OAuth app";

function steps(): Step[] {
  const sessions = region("Sessions");
  const needs = region("Needs you");
  const panel = dialog(TITLE);
  return [
    toSessions,
    { press: inside(sessions, button(/Sign in with GitHub/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, opened at the foot of the thread" },
    { hover: inside(needs, role("img", "Steps")), say: "A procedure wears the Steps glyph" },
    { hover: inside(needs, role("img", "Reply")), say: "An ordinary item keeps its own" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "A press opens its steps, not the link" },
    { look: inside(panel, button(/github\.com\/settings\/applications\/new/)), say: "The link is one key at the top" },
    { look: inside(panel, role("checkbox", "Name it Armada")), say: "Each step has a tick" },
    { press: inside(panel, role("checkbox", "Name it Armada")), say: "The first step done" },
    { press: inside(panel, role("checkbox", "Tick Enable Device Flow")), say: "The third done, the second left" },
    { press: inside(panel, role("button", /^Close( panel)?( Esc)?$/)), say: "Closed" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "Opened again" },
    { look: inside(panel, role("checkbox", "Tick Enable Device Flow")), say: "The ticks are kept" },
    { type: "Client ID is Iv1.8a61f9b3a7aba766", into: inside(panel, role("textbox", `Reply to ${TITLE}`)), say: "What is typed here answers the item" },
    { look: inside(panel, button("Send")), say: "Send answers it, and the box clears" },
  ];
}

const played = walk("needs-you-steps", steps());

export { played as "needs-you-steps" };
