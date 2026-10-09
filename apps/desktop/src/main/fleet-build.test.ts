// The build read, and the restart asked for. Fleet is a fake that records what it was asked.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { Answer } from "./request";
import { FleetBuilds, FOLLOW_MS, WATCH_MS } from "./fleet-build";

const AT_MAIN: FleetBuildReport = { on: "main", commit: "a".repeat(40), position: { ahead: 0, behind: 3 } };
const REFUSED: Answer = {
  ok: false,
  outcome: {
    ok: false,
    why: "refused",
    error: { code: "fleet.build_not_offered", message: "no served repository holds scripts/restart-build", run_id: "r", fields: {}, chain: [] },
  },
};
const UNREACHABLE: Answer = {
  ok: false,
  outcome: { ok: false, why: "transport", detail: "fetch failed", fault: { method: "GET", path: "/fleet/build", why: "unreachable" } },
};

function rig(answers: Answer[], held: FleetBuildReport | null = null, port: number | null = 40000) {
  let state: BridgeState = { ...NOTHING_YET, fleetBuild: held };
  const asked: { method: string; path: string; body: unknown }[] = [];
  const queue = [...answers];
  const builds = new FleetBuilds({
    port: () => port,
    current: () => state,
    publish: (change) => {
      state = { ...state, ...change };
    },
    ask: async (_port, method, path, body) => {
      asked.push({ method, path, body });
      return queue.shift() ?? { ok: true, body: AT_MAIN };
    },
  });
  return { builds, asked, state: () => state };
}

describe("the build Fleet runs on", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("is published as Fleet reported it", async () => {
    const { builds, state } = rig([{ ok: true, body: AT_MAIN }]);
    await builds.read(40000);
    expect(state().fleetBuild).toEqual(AT_MAIN);
  });

  it("is dropped when Fleet offers none, and kept when Fleet does not answer", async () => {
    const { builds, state } = rig([{ ok: true, body: AT_MAIN }, UNREACHABLE, REFUSED]);
    await builds.read(40000);
    await builds.read(40000);
    expect(state().fleetBuild).toEqual(AT_MAIN);
    await builds.read(40000);
    expect(state().fleetBuild).toBeNull();
  });

  it("is read once now and then every minute while the connection stands", async () => {
    const { builds, asked } = rig([]);
    builds.watch(40000);
    expect(asked).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(WATCH_MS * 2);
    expect(asked).toHaveLength(3);
    builds.close();
    await vi.advanceTimersByTimeAsync(WATCH_MS * 2);
    expect(asked).toHaveLength(3);
  });
});

describe("restarting onto another build", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("posts the build and the say-so, and marks the report restarting before the next read", async () => {
    const { builds, asked, state } = rig([{ ok: true, body: { build: "preview" } }], AT_MAIN);
    const done = await builds.change("preview", true);
    expect(done).toEqual({ ok: true });
    expect(asked[0]).toEqual({ method: "POST", path: "/fleet/build/change", body: { build: "preview", adopt: true } });
    expect(state().fleetBuild?.restarting).toBe("preview");
    builds.close();
  });

  it("changes nothing on a refusal", async () => {
    const { builds, state } = rig([REFUSED]);
    const done = await builds.change("main", false);
    expect(done).toMatchObject({ ok: false, why: "refused" });
    expect(state().fleetBuild).toBeNull();
  });

  it("is refused before it is sent when Fleet is not connected", async () => {
    const { builds, asked } = rig([], null, null);
    await expect(builds.change("main", false)).resolves.toEqual({ ok: false, why: "not_connected" });
    expect(asked).toEqual([]);
  });

  it("reads again until the restart has stopped, and then stops reading", async () => {
    const working: FleetBuildReport = { ...AT_MAIN, restarting: "main" };
    const failed: FleetBuildReport = { ...AT_MAIN, failed: "refusing: drone-3's Drone is working" };
    const { builds, asked, state } = rig(
      [
        { ok: true, body: { build: "main" } },
        { ok: true, body: working },
        { ok: true, body: failed },
      ],
      AT_MAIN,
    );
    await builds.change("main", false);
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.restarting).toBe("main");
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild).toEqual(failed);
    const reads = asked.length;
    await vi.advanceTimersByTimeAsync(FOLLOW_MS * 5);
    expect(asked).toHaveLength(reads);
  });
});
