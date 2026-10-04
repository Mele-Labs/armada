// A Prototype held at Build, its `walk` server already up for review:
// `prototype-walked`. Opening the Job opens the server in Bridge's own window,
// which a browser has none of, so `walk-window.tsx` stands in for it.

import type { ServerEntry, ServerState } from "@armada/protocol";

import type { Scenario } from "./moment";
import { openWalkWindow } from "./walk-window";

/** The mock itself on another scenario: the one address a browser page can serve as a Job's mock. */
function itself(): string {
  // The roster is also read in node, by `scenario.test.ts`, where there is no page.
  if (typeof window === "undefined") return "http://localhost:41311/?scenario=every-state&frame";
  const url = new URL(window.location.href);
  url.search = "?scenario=every-state&frame";
  return url.toString();
}

/** `base` — the Prototype Job open — over a Fleet that started its `mock` for review at the stop. */
export function walkedPrototype(base: Scenario): Scenario {
  const jobId = base.opens!;
  const link = { url: itself(), name: "Bridge on a mock Fleet" };
  const serving: ServerState = {
    id: "01SERVERPROTOTYPEMOCK00000",
    name: "mock",
    job_id: jobId,
    checkout: { path: "/repos/armada/.armada/worktrees/44", commit: "5d1e09ab", behind: 0 },
    phase: "serving",
    serve: "pnpm -C apps/desktop exec vite --config vite.mock.config.ts --port 41311 --strictPort",
    ports: [{ name: "mock", port: 41311 }],
    links: [link],
    started_by: "person",
    for_review: true,
    started_at: "2026-09-22T10:20:02Z",
    serving_since: "2026-09-22T10:20:11Z",
    stopped: false,
    log: ".armada/servers/44/01SERVERPROTOTYPEMOCK00000/output.log",
  };
  const entry: ServerEntry = {
    name: "mock",
    serve: "pnpm -C apps/desktop exec vite --config vite.mock.config.ts --port ${port.mock} --strictPort",
    ready: "curl -sf http://localhost:${port.mock}",
    links: [{ url: "http://localhost:${port.mock}", name: link.name }],
    destructive: false,
    instance: serving,
  };
  return {
    ...base,
    name: "prototype-walked",
    says: "A Prototype held at Build, its mock served for review and opened in Bridge's window",
    state: { ...base.state, servers: { servers: [serving] } },
    behaves: (fleet) => ({
      watchRunSheet: async (watched) =>
        fleet.publish({
          runSheet:
            watched === null
              ? { state: "none" }
              : {
                  state: "read",
                  jobId: watched,
                  sheet: {
                    job_id: watched,
                    setup: [],
                    checks: [],
                    commands: [],
                    servers: [entry],
                    worktree_on_disk: true,
                    worktree_differs: false,
                    drone_working: false,
                  },
                },
        }),
      openCaptureWindow: async (serverId, url) => {
        const server = fleet.state().servers.servers.find((one) => one.id === serverId);
        if (server === undefined) return { ok: false, why: "no_address" };
        openWalkWindow(server.name, server.links[0]?.url === url ? url : link.url);
        return { ok: true };
      },
    }),
  };
}
