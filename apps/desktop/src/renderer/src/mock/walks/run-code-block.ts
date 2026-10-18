// A shell block in a Session's thread, run from where it is read. Hover shows Run, the output streams
// under the block, one run fails, a long one is stopped, and a block runs again. Over the `run-code-block`
// scenario.

import { kit, toSessions } from "../sessions/walk-kit";
import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const { thread } = kit(false);
const sessions = region("Jobs", { exact: true });
const block = (command: string) => inside(thread, role("group", command));
const typecheck = block("pnpm -C apps/desktop typecheck");
const tests = block("cd packages/components");
const tail = block("tail -f build.log");

const steps: Step[] = [
  { press: inside(sessions, button(/Verify the build/)), say: "A Job whose agent gave three commands" },
  { hover: text("pnpm -C apps/desktop typecheck"), say: "Hovering a shell block shows Run" },
  { press: inside(typecheck, button("Run", { exact: true })), say: "Run, in the Job's directory" },
  { look: inside(typecheck, text(/Found 0 errors/)), say: "The output streams in right under the block" },
  { look: inside(typecheck, role("img", "Passed")), say: "It passed; the exit code is its tooltip" },
  { look: inside(typecheck, button("Run again")), say: "Run again sits where Run was" },
  { press: inside(tests, button("Run", { exact: true })), say: "The multi-line block runs the same way" },
  { look: inside(tests, text(/1 failed/)), say: "This one fails" },
  { look: inside(tests, role("img", "Failed")), say: "A failed mark, with the exit code on its tooltip" },
  { press: inside(tail, button("Run", { exact: true })), say: "The console block runs without its prompt" },
  { look: inside(tail, button("Stop")), say: "It does not end; Stop is on the block" },
  { press: inside(tail, button("Stop")), say: "Stopped" },
  { press: inside(tail, button("Run again")), say: "And run again" },
];

const runCodeBlock = walk("run-code-block", [toSessions, ...steps]);

export { runCodeBlock };
