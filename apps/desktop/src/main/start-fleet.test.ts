// Start Fleet runs `armada fleet start` and says what it came to; Reopen Armada asks launchd for this
// Bridge's own job and does not wait on it. The commands are fakes that record what they were asked.

import { mkdirSync, mkdtempSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  armadaIn,
  keptBuildsOf,
  reopenBridge,
  startFleet,
  type Detach,
  type Exec,
} from "./start-fleet";

function ran(
  code: number,
  stderr = "",
): {
  exec: Exec;
  asked: [
    string,
    string[],
  ][];
} {
  const asked: [
    string,
    string[],
  ][] = [];
  return {
    asked,
    exec: async (file, args) => {
      asked.push([
        file,
        args,
      ]);
      return {
        code,
        stderr,
      };
    },
  };
}

describe("starting Fleet from the window", () => {
  it("runs armada fleet start and is ok when it exits clean", async () => {
    const { exec, asked } = ran(0);
    await expect(startFleet(null, exec, "/kept/current/armada")).resolves.toEqual({
      ok: true,
    });
    expect(asked).toEqual([
      [
        "/kept/current/armada",
        [
          "fleet",
          "start",
        ],
      ],
    ]);
  });

  it("passes the command's own error on when it exits with one", async () => {
    const { exec } = ran(1, "Bootstrap failed: 5: Input/output error\n");
    await expect(startFleet(null, exec, "armada")).resolves.toEqual({
      ok: false,
      why: "refused",
      detail: "Bootstrap failed: 5: Input/output error",
    });
  });

  it("names the exit code when the command said nothing", async () => {
    const { exec } = ran(2);
    await expect(startFleet(null, exec, "armada")).resolves.toEqual({
      ok: false,
      why: "refused",
      detail: "armada fleet start exited 2",
    });
  });

  it("runs the armada of the current kept build, and the one on PATH where there is none", () => {
    const builds = mkdtempSync(join(tmpdir(), "builds-"));
    expect(armadaIn(builds)).toBe("armada");
    expect(armadaIn(null)).toBe("armada");
    mkdirSync(join(builds, "2026-11-13T004852Z-126b4bb"));
    writeFileSync(join(builds, "2026-11-13T004852Z-126b4bb", "armada"), "");
    symlinkSync("2026-11-13T004852Z-126b4bb", join(builds, "current"));
    expect(armadaIn(builds)).toBe(join(builds, "current", "armada"));
  });
});

describe("reopening this Bridge", () => {
  it("kickstarts the Bridge job of the Fleet label, and is ok once the command is away", async () => {
    const asked: [
      string,
      string[],
    ][] = [];
    const start: Detach = async (file, args) => {
      asked.push([
        file,
        args,
      ]);
      return null;
    };
    await expect(reopenBridge(start, {}, 501)).resolves.toEqual({
      ok: true,
    });
    await reopenBridge(
      start,
      {
        ARMADA_FLEET_LABEL: "com.armada.dev",
      },
      501,
    );
    expect(
      asked.map(([file, args]) => [
        file,
        args.at(-1),
      ]),
    ).toEqual([
      [
        "launchctl",
        "gui/501/com.armada.fleet.bridge",
      ],
      [
        "launchctl",
        "gui/501/com.armada.dev.bridge",
      ],
    ]);
    expect(asked[0]?.[1].slice(0, 2)).toEqual([
      "kickstart",
      "-k",
    ]);
  });

  it("says why when the command would not start", async () => {
    await expect(reopenBridge(async () => "spawn launchctl ENOENT", {}, 501)).resolves.toEqual({
      ok: false,
      detail: "spawn launchctl ENOENT",
    });
  });
});

describe("the kept builds this Bridge and the install are", () => {
  const support = mkdtempSync(join(tmpdir(), "support-"));
  const builds = join(support, "builds");
  mkdirSync(join(builds, "old", "bridge"), {
    recursive: true,
  });
  mkdirSync(join(builds, "new", "bridge"), {
    recursive: true,
  });
  symlinkSync("new", join(builds, "current"));

  it("names this Bridge's build folder and the one current points at", () => {
    expect(keptBuildsOf(join(builds, "old", "bridge"), builds)).toEqual({
      bridge: "old",
      current: "new",
    });
    expect(keptBuildsOf(join(builds, "current", "bridge"), builds)).toEqual({
      bridge: "new",
      current: "new",
    });
  });

  it("names no build for a Bridge run from a tree, or where there are no kept builds", () => {
    expect(keptBuildsOf(join(support, "tree", "apps", "desktop"), builds).bridge).toBeNull();
    expect(keptBuildsOf(join(builds, "old", "bridge"), join(support, "none"))).toEqual({
      bridge: null,
      current: null,
    });
    expect(keptBuildsOf(join(builds, "old", "bridge"), null)).toEqual({
      bridge: null,
      current: null,
    });
  });
});
