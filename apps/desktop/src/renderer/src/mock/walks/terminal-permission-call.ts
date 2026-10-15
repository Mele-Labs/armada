// A Session in a terminal stopped on its harness's own permission prompt is a call in the Cockpit,
// and only the terminal answers it: the call's one act opens the Session. Over `terminal-permission-call`.

import { button, inside, region, role, text, walk } from "../walk";

const cockpit = region("Dashboard");
const card = region(/^Session: Port the cache to sqlite/);

const terminalPermissionCall = walk("terminal-permission-call", [
  { look: card, say: "A terminal Session stopped on its own permission prompt, with nobody at the terminal" },
  { look: inside(card, text("Bash cargo build --release")), say: "The tool and its command, as the terminal asks it" },
  { key: "Enter", on: cockpit, say: "Only the terminal can answer it, so Enter opens the Session" },
  { hover: inside(region("Needs you"), role("img", "Bash cargo build --release")), say: "The Session keeps it under Needs you at the foot of its thread" },
  { press: inside(region("Needs you"), button(/Needs you/)), say: "Open the band" },
  { look: inside(region("Needs you"), text("Bash cargo build --release")), say: "The prompt, as on the call" },
]);

export { terminalPermissionCall };
