// The build read, and the restart asked for. Fleet is a fake that records what it was asked.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FleetBuildReport } from "@armada/protocol";
import { NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { Answer } from "./request";
import { FleetBuilds, FOLLOW_MS, WATCH_MS } from "./fleet-build";

const AT_MAIN: FleetBuildReport = {
  on: "main",
  commit: "a".repeat(40),
  position: {
    ahead: 0,
    behind: 3,
  },
};
const REFUSED: Answer = {
  ok: false,
  outcome: {
    ok: false,
    why: "refused",
    error: {
      code: "fleet.build_not_offered",
      message: "no served repository holds scripts/restart-build",
      run_id: "r",
      fields: {},
      chain: [],
    },
  },
};
const UNREACHABLE: Answer = {
  ok: false,
  outcome: {
    ok: false,
    why: "transport",
    detail: "fetch failed",
    fault: {
      method: "GET",
      path: "/fleet/build",
      why: "unreachable",
    },
  },
};

function rig(answers: Answer[], held: FleetBuildReport | null = null, port: number | null = 40000) {
  let state: BridgeState = {
    ...NOTHING_YET,
    fleetBuild: held,
  };
  const asked: {
    method: string;
    path: string;
    body: unknown;
  }[] = [];
  const queue = [
    ...answers,
  ];
  const builds = new FleetBuilds({
    port: () => port,
    current: () => state,
    publish: (change) => {
      state = {
        ...state,
        ...change,
      };
    },
    ask: async (_port, method, path, body) => {
      asked.push({
        method,
        path,
        body,
      });
      return (
        queue.shift() ?? {
          ok: true,
          body: AT_MAIN,
        }
      );
    },
  });
  return {
    builds,
    asked,
    state: () => state,
  };
}

describe("the build Fleet runs on", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("is published as Fleet reported it", async () => {
    const { builds, state } = rig([
      {
        ok: true,
        body: AT_MAIN,
      },
    ]);
    await builds.read(40000);
    expect(state().fleetBuild).toEqual(AT_MAIN);
  });

  it("is dropped when Fleet offers none, and kept when Fleet does not answer", async () => {
    const { builds, state } = rig([
      {
        ok: true,
        body: AT_MAIN,
      },
      UNREACHABLE,
      REFUSED,
    ]);
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
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            build: "preview",
          },
        },
      ],
      AT_MAIN,
    );
    const done = await builds.change("preview", true);
    expect(done).toEqual({
      ok: true,
    });
    expect(asked[0]).toEqual({
      method: "POST",
      path: "/fleet/build/change",
      body: {
        build: "preview",
        adopt: true,
      },
    });
    expect(state().fleetBuild?.restarting).toBe("preview");
    builds.close();
  });

  it("changes nothing on a refusal", async () => {
    const { builds, state } = rig([
      REFUSED,
    ]);
    const done = await builds.change("main", false);
    expect(done).toMatchObject({
      ok: false,
      why: "refused",
    });
    expect(state().fleetBuild).toBeNull();
  });

  it("is refused before it is sent when Fleet is not connected", async () => {
    const { builds, asked } = rig([], null, null);
    await expect(builds.change("main", false)).resolves.toEqual({
      ok: false,
      why: "not_connected",
    });
    expect(asked).toEqual([]);
  });

  it("reads again until the restart has stopped, and then stops reading", async () => {
    const working: FleetBuildReport = {
      ...AT_MAIN,
      restarting: "main",
    };
    const failed: FleetBuildReport = {
      ...AT_MAIN,
      failed: "refusing: drone-3's Drone is working",
    };
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            build: "main",
          },
        },
        {
          ok: true,
          body: working,
        },
        {
          ok: true,
          body: failed,
        },
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

describe("the stage of a restart", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("is published as Fleet reported it", async () => {
    const working: FleetBuildReport = {
      ...AT_MAIN,
      restarting: "preview",
      stage: "building_bridge",
    };
    const { builds, state } = rig([
      {
        ok: true,
        body: working,
      },
    ]);
    await builds.read(40000);
    expect(state().fleetBuild?.stage).toBe("building_bridge");
  });

  it("moves with each read of a restart, and is gone once the restart is", async () => {
    const at = (stage: FleetBuildReport["stage"]): FleetBuildReport => ({
      ...AT_MAIN,
      restarting: "main",
      ...(stage === undefined
        ? {}
        : {
            stage,
          }),
    });
    const { builds, state } = rig(
      [
        {
          ok: true,
          body: {
            build: "main",
          },
        },
        {
          ok: true,
          body: at("fetching_main"),
        },
        {
          ok: true,
          body: at("building_fleet"),
        },
        {
          ok: true,
          body: AT_MAIN,
        },
      ],
      AT_MAIN,
    );
    await builds.change("main", false);
    expect(state().fleetBuild?.stage).toBeUndefined();
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.stage).toBe("fetching_main");
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.stage).toBe("building_fleet");
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild).toEqual(AT_MAIN);
    builds.close();
  });
});

describe("reverting to the build before", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const KEPT: FleetBuildReport = {
    ...AT_MAIN,
    previous: {
      built_at: "2026-11-05T16:20:00Z",
      commit: "b".repeat(40),
    },
  };

  it("posts no body, and marks the report reverting with the old failure gone", async () => {
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            commit: "b".repeat(40),
          },
        },
      ],
      {
        ...KEPT,
        failed: "launchd would not load",
      },
    );
    const done = await builds.revert();
    expect(done).toEqual({
      ok: true,
    });
    expect(asked[0]).toEqual({
      method: "POST",
      path: "/fleet/build/revert",
      body: undefined,
    });
    expect(state().fleetBuild?.reverting).toBe(true);
    expect(state().fleetBuild).not.toHaveProperty("failed");
    builds.close();
  });

  it("says why a refusal came where a failed restart is said, and does not mark it reverting", async () => {
    const { builds, state } = rig(
      [
        {
          ok: false,
          outcome: {
            ok: false,
            why: "refused",
            error: {
              code: "fleet.build_not_revertible",
              message: "No build before this one is kept",
              run_id: "r",
              fields: {},
              chain: [],
            },
          },
        },
      ],
      KEPT,
    );
    const done = await builds.revert();
    expect(done).toMatchObject({
      ok: false,
      why: "refused",
    });
    expect(state().fleetBuild?.failed).toBe("No build before this one is kept");
    expect(state().fleetBuild?.reverting).toBeUndefined();
  });

  it("is refused before it is sent when Fleet is not connected", async () => {
    const { builds, asked } = rig([], KEPT, null);
    await expect(builds.revert()).resolves.toEqual({
      ok: false,
      why: "not_connected",
    });
    expect(asked).toEqual([]);
  });

  it("reads again until the revert has stopped, and then stops reading", async () => {
    const working: FleetBuildReport = {
      ...KEPT,
      reverting: true,
    };
    const done: FleetBuildReport = {
      ...AT_MAIN,
    };
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            commit: "b".repeat(40),
          },
        },
        {
          ok: true,
          body: working,
        },
        {
          ok: true,
          body: done,
        },
      ],
      KEPT,
    );
    await builds.revert();
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.reverting).toBe(true);
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild).toEqual(done);
    const reads = asked.length;
    await vi.advanceTimersByTimeAsync(FOLLOW_MS * 5);
    expect(asked).toHaveLength(reads);
  });
});

describe("picking Jobs for the snapshot", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const BUILDING: FleetBuildReport = {
    ...AT_MAIN,
    picked: [
      {
        branch: "armada/one",
        title: "One",
      },
    ],
    snapshot: {
      state: "building",
      stage: "building_fleet",
    },
  };

  it("posts the pick as it was made, then reads the report so the chips are drawn at once", async () => {
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            picked: [
              "armada/one",
            ],
          },
        },
        {
          ok: true,
          body: BUILDING,
        },
      ],
      AT_MAIN,
    );
    const done = await builds.pick({
      session_id: "s-1",
      picked: true,
    });
    expect(done).toEqual({
      ok: true,
    });
    expect(asked[0]).toEqual({
      method: "POST",
      path: "/fleet/build/pick",
      body: {
        session_id: "s-1",
        picked: true,
      },
    });
    expect(asked[1]?.path).toBe("/fleet/build");
    expect(state().fleetBuild).toEqual(BUILDING);
    builds.close();
  });

  it("reads again while the snapshot builds, and stops once it is ready", async () => {
    const ready: FleetBuildReport = {
      ...BUILDING,
      snapshot: {
        state: "ready",
        jobs: [
          "One",
        ],
      },
    };
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            picked: [
              "armada/one",
            ],
          },
        },
        {
          ok: true,
          body: BUILDING,
        },
        {
          ok: true,
          body: BUILDING,
        },
        {
          ok: true,
          body: ready,
        },
      ],
      AT_MAIN,
    );
    await builds.pick({
      branch: "armada/one",
      picked: true,
    });
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.snapshot?.state).toBe("building");
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild).toEqual(ready);
    const reads = asked.length;
    await vi.advanceTimersByTimeAsync(FOLLOW_MS * 5);
    expect(asked).toHaveLength(reads);
  });

  it("hands a refusal back unchanged and leaves the report without a failed line", async () => {
    const { builds, state } = rig(
      [
        {
          ok: false,
          outcome: {
            ok: false,
            why: "refused",
            error: {
              code: "fleet.build_no_branch",
              message: "no branch",
              run_id: "r",
              fields: {},
              chain: [],
            },
          },
        },
      ],
      AT_MAIN,
    );
    const done = await builds.pick({
      session_id: "s-1",
      picked: true,
    });
    expect(done).toMatchObject({
      ok: false,
      why: "refused",
      error: {
        code: "fleet.build_no_branch",
      },
    });
    expect(state().fleetBuild).toEqual(AT_MAIN);
  });

  it("is refused before it is sent when Fleet is not connected", async () => {
    const { builds, asked } = rig([], AT_MAIN, null);
    await expect(
      builds.pick({
        branch: "armada/one",
        picked: false,
      }),
    ).resolves.toEqual({
      ok: false,
      why: "not_connected",
    });
    expect(asked).toEqual([]);
  });
});

describe("swapping onto the snapshot", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const READY: FleetBuildReport = {
    ...AT_MAIN,
    snapshot: {
      state: "ready",
      jobs: [
        "One",
      ],
    },
  };

  it("posts no body, and marks the report swapping with the old failure gone", async () => {
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            commit: "c".repeat(40),
          },
        },
      ],
      {
        ...READY,
        failed: "Swapped back: the new Fleet did not answer",
      },
    );
    expect(await builds.swap()).toEqual({
      ok: true,
    });
    expect(asked[0]).toEqual({
      method: "POST",
      path: "/fleet/build/swap",
      body: undefined,
    });
    expect(state().fleetBuild?.swapping).toBe(true);
    expect(state().fleetBuild).not.toHaveProperty("failed");
    builds.close();
  });

  it("says why a refusal came where a failed restart is said, and does not mark it swapping", async () => {
    const { builds, state } = rig(
      [
        {
          ok: false,
          outcome: {
            ok: false,
            why: "refused",
            error: {
              code: "fleet.build_not_swappable",
              message: "No snapshot is kept",
              run_id: "r",
              fields: {},
              chain: [],
            },
          },
        },
      ],
      READY,
    );
    await builds.swap();
    expect(state().fleetBuild?.failed).toBe("No snapshot is kept");
    expect(state().fleetBuild?.swapping).toBeUndefined();
  });

  it("stays swapping while Fleet does not answer, and reads on until a Fleet answers without it", async () => {
    const back: FleetBuildReport = {
      ...AT_MAIN,
      failed: "Swapped back: the new Fleet did not answer in a minute",
    };
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            commit: "c".repeat(40),
          },
        },
        UNREACHABLE,
        UNREACHABLE,
        {
          ok: true,
          body: back,
        },
      ],
      READY,
    );
    await builds.swap();
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.swapping).toBe(true);
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild?.swapping).toBe(true);
    await vi.advanceTimersByTimeAsync(FOLLOW_MS);
    expect(state().fleetBuild).toEqual(back);
    const reads = asked.length;
    await vi.advanceTimersByTimeAsync(FOLLOW_MS * 5);
    expect(asked).toHaveLength(reads);
  });
});

describe("keeping the Jobs tried", () => {
  it("posts no body and reads the report, so trying is empty", async () => {
    const { builds, asked, state } = rig(
      [
        {
          ok: true,
          body: {
            told: [
              "One",
            ],
          },
        },
        {
          ok: true,
          body: AT_MAIN,
        },
      ],
      {
        ...AT_MAIN,
        trying: [
          "One",
        ],
      },
    );
    expect(await builds.keep()).toEqual({
      ok: true,
    });
    expect(asked[0]).toEqual({
      method: "POST",
      path: "/fleet/build/keep",
      body: undefined,
    });
    expect(state().fleetBuild).toEqual(AT_MAIN);
  });

  it("hands a refusal back and leaves what trying says", async () => {
    const { builds, state } = rig(
      [
        UNREACHABLE,
      ],
      {
        ...AT_MAIN,
        trying: [
          "One",
        ],
      },
    );
    const done = await builds.keep();
    expect(done).toMatchObject({
      ok: false,
      why: "transport",
    });
    expect(state().fleetBuild?.trying).toEqual([
      "One",
    ]);
  });

  it("is refused before it is sent when Fleet is not connected", async () => {
    const { builds, asked } = rig([], AT_MAIN, null);
    await expect(builds.keep()).resolves.toEqual({
      ok: false,
      why: "not_connected",
    });
    expect(asked).toEqual([]);
  });
});
