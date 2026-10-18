// A block with no language that reads as shell gets Run like a tagged one; code and a diagram get none.
// A slash command the terminal lists, in inline code, is sent to its terminal. Over the
// `run-untagged-and-slash` scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const sessions = region("Jobs", { exact: true });
const chain = inside(thread, role("group", /^cd ~\/Development\/armada/));
const snippet = inside(thread, text(/const rows = items\.filter/));
const diagram = inside(thread, text(/│ Bridge │/));

const steps: Step[] = [
  { press: inside(sessions, button(/Clean up the merged slot/)), say: "A terminal Job whose agent gave a command, some code and a diagram" },
  { hover: text(/cd ~\/Development\/armada/), say: "A block with no language that reads as shell shows Run on hover" },
  { press: inside(chain, button("Run", { exact: true })), say: "Run, in the Job's directory" },
  { look: inside(chain, role("img", "Passed")), say: "The output and the exit code sit under it, as for a tagged block" },
  { hover: snippet, say: "A code block with no language reads as code: nothing to run" },
  { hover: diagram, say: "Nor does a diagram get a Run" },
  { hover: role("button", "/update-armada"), say: "A slash command in inline code shows a small play on hover" },
  { press: role("button", "/update-armada"), say: "A press types it into this Job's terminal" },
  { look: role("img", "Sent"), say: "Sent. The command runs in the terminal, so nothing comes back here" },
  { look: role("button", "/reload-plugins"), say: "The next one waits to be run when its turn comes" },
  { hover: text("/tmp"), say: "/tmp is not a command the terminal lists, so it stays plain code" },
];

const runUntaggedAndSlash = walk("run-untagged-and-slash", [toSessions, ...steps]);

export { runUntaggedAndSlash };
