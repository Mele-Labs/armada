// A prototype walked inside Bridge: a Prototype Job held at Build serves its
// own worktree's mock Fleet from the run sheet, and **Walk in Bridge** opens it
// in Bridge's own window rather than the system browser. The window is the
// capture window with no Studio, `docs/practices/capture-window.md`.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { ServerEntry, ServerState } from "@armada/protocol";
import { prototypeKind } from "@armada/screens/src/fixtures/build/kinds";

import type { BridgeState } from "../../../shared/bridge";
import type { FleetHandle } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const fixture = prototypeKind();

/** This Job's `mock`, as Fleet holds it once it answers. */
const SERVING: ServerState = {
  id: "01SERVERPROTOTYPEMOCK00000",
  name: "mock",
  job_id: fixture.job.id,
  checkout: { path: "/repos/armada/.armada/worktrees/44", commit: "5d1e09ab", behind: 0 },
  phase: "serving",
  serve: "pnpm -C apps/desktop exec vite --config vite.mock.config.ts --port 41311 --strictPort",
  ports: [{ name: "mock", port: 41311 }],
  links: [{ url: "http://localhost:41311", name: "Bridge on a mock Fleet" }],
  started_by: "person",
  started_at: "2026-09-22T10:21:00Z",
  serving_since: "2026-09-22T10:21:09Z",
  stopped: false,
  log: ".armada/servers/44/01SERVERPROTOTYPEMOCK00000/output.log",
};

const MOCK: ServerEntry = {
  name: "mock",
  serve: "pnpm -C apps/desktop exec vite --config vite.mock.config.ts --port ${port.mock} --strictPort",
  ready: "curl -sf http://localhost:${port.mock}",
  links: [{ url: "http://localhost:${port.mock}", name: "Bridge on a mock Fleet" }],
  destructive: false,
  instance: SERVING,
};

const SHEET: BridgeState["runSheet"] = {
  state: "read",
  jobId: fixture.job.id,
  sheet: {
    job_id: fixture.job.id,
    setup: [],
    checks: [],
    commands: [],
    servers: [MOCK],
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: false,
  },
};

async function opened() {
  const scenario = onJob(fixture);
  const app = mount({
    ...scenario,
    behaves: (fleet: FleetHandle) => ({
      watchRunSheet: async (jobId) => fleet.publish({ runSheet: jobId === null ? { state: "none" } : SHEET }),
    }),
  });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  await userEvent.keyboard("r");
  await page.getByRole("dialog", { name: "Run" }).getByRole("button", { name: /^mock/ }).click();
  return app.api;
}

test("a prototype's mock opens in Bridge's own window, on its own link", async () => {
  const api = await opened();
  const openCaptureWindow = vi.spyOn(api, "openCaptureWindow");
  const openServerLink = vi.spyOn(api, "openServerLink");

  await page.getByRole("button", { name: "Walk in Bridge" }).click();

  await expect.poll(() => openCaptureWindow.mock.calls.length).toBe(1);
  expect(openCaptureWindow).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
  expect(openServerLink).not.toHaveBeenCalled();
});

test("the link itself still goes to the system browser", async () => {
  const api = await opened();
  const openServerLink = vi.spyOn(api, "openServerLink");

  await page.getByRole("button", { name: "Bridge on a mock Fleet" }).click();

  await expect.poll(() => openServerLink.mock.calls.length).toBe(1);
  expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
});
