// The Processes section of a Job's Artifacts layer: three rows, and Stop on the walk server turns it to stopped. Over the `processes` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const artifacts = region("Artifacts", { exact: true });
const processes = inside(artifacts, region("Processes", { exact: true }));

const stopping = walk("processes", [
  { press: deckRow("Queue session for later"), say: "A Job on the Cockpit" },
  { press: button("Artifacts", { exact: true }), say: "Artifacts holds what the Job made and started" },
  { look: processes, say: "Processes: the walk server on port 5501, a dev Fleet and a stopped watcher, each with its pid" },
  { press: inside(processes, button("Stop vite --port 5501")), say: "Stop on the walk server" },
  { look: inside(processes, role("img", "Stopped", { exact: true })), say: "Its row reads stopped, and the Stop button is gone" },
]);

export { stopping as "processes" };
