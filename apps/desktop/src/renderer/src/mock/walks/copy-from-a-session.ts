// Copy on a fenced block in a Session's thread. A command block has Copy beside Run, a plain block has Copy
// alone, and a press shows a check for a moment. The clipboard is the walk's own: what was copied is shown in
// a panel. Over the `copy-from-a-session` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const sessions = region("Jobs", { exact: true });
const pasted = region("What they paste");
const command = inside(thread, role("group", "cargo test -p store store_open"));

const steps: Step[] = [
  { press: inside(sessions, button(/Find the flaky test/)), say: "A Job whose agent gave a command and a block of readings" },
  { hover: text("cargo test -p store store_open"), say: "Hovering the command shows Copy beside Run" },
  { look: inside(command, button("Copy", { exact: true })), say: "Copy sits in the same group as Run" },
  { hover: inside(command, button("Copy", { exact: true })), say: "It is named on hover" },
  { press: inside(command, button("Copy", { exact: true })), say: "Copy writes the command as it stands" },
  { look: inside(command, button("Copied")), say: "A check says it is done, then Copy returns" },
  { look: inside(pasted, text("cargo test -p store store_open")), say: "What they paste" },
  { press: inside(pasted, button("Close")), say: "Closed" },
  { hover: text("read 2 -> 12:00:01.01"), say: "A plain block has Copy too, top right, clear of the text" },
  { look: inside(thread, button("Copy", { exact: true })), say: "Copy alone, with no Run" },
  { press: inside(thread, button("Copy", { exact: true })), say: "It copies the block" },
  { look: inside(thread, button("Copied")), say: "The same check" },
  { look: inside(pasted, text("read 1 -> 12:00:00.99")), say: "Both lines, as written" },
];

const copyFromASession = walk("copy-from-a-session", [toSessions, ...steps]);

export { copyFromASession };
