// The System layer of a Job: processes, CPU, memory, disk, worktrees and logs, and Stop on the walk server turns its row to stopped. Over the `processes` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const system = region("System", { exact: true });
const section = (name: string) => inside(system, region(name, { exact: true }));
const processes = section("Processes");
const walkServer = inside(processes, role("listitem", /vite --port 5501/));

const stopping = walk("processes", [
  { press: deckRow("Queue session for later"), say: "A Job on the Cockpit" },
  { press: button("System", { exact: true }), say: "System holds what the Job runs and what it costs the machine" },
  { look: processes, say: "Processes: the walk server on port 5501, a dev Fleet and a stopped watcher, each with its pid" },
  { look: section("CPU"), say: "CPU reading with its bar" },
  { look: section("Memory"), say: "Memory reading with its bar" },
  { look: section("Disk"), say: "Disk reading with its bar" },
  { look: section("Worktrees"), say: "The two worktrees the Job holds, each with its branch" },
  { look: section("Logs"), say: "Three log files with a size each" },
  { press: inside(processes, button("Stop vite --port 5501")), say: "Stop on the walk server" },
  { look: inside(walkServer, role("img", "Stopped", { exact: true })), say: "Its row reads stopped, and its Stop button is gone" },
]);

export { stopping as "processes" };
