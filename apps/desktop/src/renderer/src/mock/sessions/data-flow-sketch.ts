// The hub an agent drew for how data moves through Fleet: converging labelled arrows into one box, a short arrow between two
// boxes, and a body longer than the `h` it asked for. What a sketch looked like when its words piled up and its last line was cut.
// Mock only, as `sketches.ts` is.

import { arrow, box, label, scene } from "../scenes";

export const DATA_FLOW_TITLE = "How data moves to and from the Fleet";

const tall = (id: string, x: number, y: number, h: number, title: string, body: string, more: Parameters<typeof box>[5]) => ({ ...box(id, x, y, title, body, more), w: 240, h });

export const DATA_FLOW = scene(
  [
    label("you", 118, -80, "you"),
    tall("bridge", 0, 0, 100, "Bridge", "Electron, apps/desktop.\nOne connection, never talks to a Drone", { icon: "globe", step: 2 }),
    tall("sessions", 520, 0, 100, "Agent sessions", "armada-fleet MCP. No stream: poll with a cursor", { icon: "bot", step: 9 }),
    tall("check", -420, 300, 100, "Check", "The repo's own Check, run in the Job's worktree", { icon: "hammer", step: 5 }),
    tall("fleet", 0, 300, 100, "Fleet", "Rust daemon: fleet + api + ipc.\nProtocol ID must match Bridge's or no socket opens; /v0 lifeboat stays up for a refused Bridge", { icon: "server", step: 3 }),
    tall("sqlite", 520, 300, 100, "SQLite", "crates/store", { icon: "hard-drive", step: 7 }),
    tall("drone-a", -120, 590, 90, "Drone", "Own worktree, --strict-mcp-config, no push", { icon: "terminal", step: 4 }),
    tall("drone-b", 240, 590, 90, "Drone", "Own worktree, --strict-mcp-config, no push", { icon: "terminal", step: 4 }),
  ],
  [
    arrow("you", "bridge", undefined, undefined, 1),
    arrow("bridge", "fleet", "WS /jobs/:id/observe and run observe, HTTP for commands", undefined, 8),
    arrow("sessions", "fleet", "MCP: get_events_since(upto), get_job, start_run, show_sketch", undefined, 9),
    arrow("fleet", "check", "code: advances, retry, fails", undefined, 5),
    arrow("fleet", "sqlite", "every transition", undefined, 7),
    arrow("drone-a", "fleet", "evidence via Armada's tool", undefined, 4),
    arrow("drone-b", "fleet", "evidence via Armada's tool", undefined, 4),
  ],
);
