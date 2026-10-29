// A Job whose agent waits on a subagent shows what the subagent is doing: the Agent call is a row with its type, its task,
// its last call and how long it has run, a press opens its own thread, and messages sent meanwhile are marked queued.
// Stop cuts it off and the marks go. Over the `foreground-subagent` scenario. armada-d3h.1.

import { button, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

const TASK = "Make the Job browser thread resizable";

function steps(): Step[] {
  const { thread, sheet, close } = kit(false);
  const sessions = region("Jobs", { exact: true });
  const row = (state: string) => inside(thread, button(`Subagent ${TASK}, ${state}`));
  const panel = sheet(`Subagent ${TASK}`);
  const queued = inside(thread, role("img", "Queued until the subagent returns"));
  return [
    { press: inside(sessions, button("Resizable Job browser thread")), say: "A Job whose agent handed the work to a subagent and waits on it" },
    { look: row("running"), say: "The Agent call is a row of its own: the subagent's type, its task, and how long it has run, ticking" },
    { look: inside(thread, text("Bash pnpm exec vitest run src/renderer/src/job-browser")), say: "Under it, the subagent's latest call. Its calls are never rows of the agent's thread" },
    { press: row("running"), say: "A press opens the subagent's own thread" },
    { look: inside(panel, text("The handle is in. Running the job-browser tests.")), say: "Everything it has done so far, as the Subagents layer shows it" },
    { press: close(panel), say: "Back to the Job" },
    { look: inside(thread, text("What are you doing?")), say: "Two messages sent while it worked" },
    { hover: queued, say: "Each is marked queued: the agent reads them when the subagent returns" },
    { press: inside(thread, button("Stop")), say: "Stop ends the turn" },
    { look: row("cut off"), say: "The row settles as cut off, with the time it ran" },
    { look: inside(thread, text("Hello?")), say: "The messages are no longer queued: the agent has them" },
  ];
}

const wide = walk("foreground-subagent", [toSessions, ...steps()]);

export { wide as "foreground-subagent" };
