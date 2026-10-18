// A Session in a terminal stopped on its harness's own permission prompt is a call in the Cockpit,
// answered from Bridge: the call offers Allow once and Refuse, and in the Session ▶ on the command is
// Allow once. Over `terminal-permission-call`.

import { button, inside, region, role, text, walk } from "../walk";

const card = region(/^Job: Port the cache to sqlite/);
const ask = role("article", "Waiting on you");

const terminalPermissionCall = walk("terminal-permission-call", [
  { look: card, say: "A terminal Job stopped on its own permission prompt, with nobody at the terminal" },
  { look: inside(card, text("Bash cargo build --release")), say: "The tool and its command, as the terminal asks it" },
  { look: inside(card, role("radio", /Allow once/)), say: "Answered here, as a hosted Job's permission is" },
  { press: button(/Open Job/), say: "Open the Job" },
  { press: inside(region("Needs you"), button(/Open cargo build --release/)), say: "The permission is a row in the panel, and a press opens it" },
  { look: inside(ask, text("cargo build --release")), say: "The command waits as a block to run" },
  { hover: inside(ask, button("Allow once")), say: "▶ is Allow once" },
  { press: inside(ask, button("Allow once")), say: "Run it: the terminal's prompt closes and the command runs there" },
  { look: region("Thread"), say: "Nothing waits on him now" },
]);

export { terminalPermissionCall };
