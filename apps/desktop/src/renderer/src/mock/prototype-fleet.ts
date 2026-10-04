// A Prototype held at Build, its `walk` server already up for review:
// `prototype-walked`. Opening the Job opens the server in Bridge's own window,
// which a browser has none of, so `walk-window.tsx` stands in for it.

import type { ServerEntry, ServerState, WalkNote } from "@armada/protocol";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { FleetHandle, Scenario } from "./moment";
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
/**
 * `fixture` with its evidence read and empty — no step has handed anything in
 * yet, which is a real answer. **The gate's decision waits on that read**, and
 * a light fixture leaves it unread, so the decision would never draw.
 */
export function evidenceRead(fixture: JobFixture): JobFixture {
  return {
    ...fixture,
    recorded: { ...fixture.recorded, evidence: { state: "read", jobId: fixture.job.id, steps: [] } },
  };
}

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
        const handle = fleet.state().jobs.find((one) => one.id === jobId)?.handle ?? jobId;
        openWalkWindow(server.name, server.links[0]?.url === url ? url : link.url, {
          job: handle,
          onNote: (said, picked) =>
            noted(fleet, base, jobId, (was) => [
              ...was,
              { id: `01WALKNOTE${String(was.length + 1).padStart(16, "0")}`, said, at: new Date().toISOString(), ...picked },
            ]),
        });
        return { ok: true };
      },
      // Sent back: the notes that went are marked, and the Job goes to work again.
      requestChanges: async (asked, _note, withWalkNotes) => {
        if (withWalkNotes === true) noted(fleet, base, asked, (was) => was.map((one) => ({ ...one, sent: true })));
        return { ok: true };
      },
      removeWalkNote: async (asked, noteId) => {
        noted(fleet, base, asked, (was) => was.filter((one) => one.id !== noteId));
        return { ok: true };
      },
    }),
  };
}

/**
 * The Job's walk notes, changed: on the Job open now, and on its own read so a
 * Job opened again still holds them — Fleet keeps them, not the window.
 */
function noted(fleet: FleetHandle, base: Scenario, jobId: string, change: (was: WalkNote[]) => WalkNote[]): void {
  const watched = fleet.state().watched;
  const kept = base.reads[jobId];
  if (kept !== undefined && kept.watched.state === "read") {
    kept.watched = { ...kept.watched, detail: { ...kept.watched.detail, walk_notes: change(kept.watched.detail.walk_notes ?? []) } };
  }
  if (watched.state === "read" && watched.jobId === jobId) {
    fleet.publish({ watched: { ...watched, detail: { ...watched.detail, walk_notes: change(watched.detail.walk_notes ?? []) } } });
  }
}
