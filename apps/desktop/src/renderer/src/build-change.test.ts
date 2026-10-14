// A restart handed to the person as a command, read as the Fleet panel's build switch.

import { describe, expect, it } from "vitest";

import { buildChangeOf, restartOf } from "./build-change";

describe("a restart command read as the build switch", () => {
  it("reads the preview restart as Refresh preview", () => {
    expect(buildChangeOf("scripts/preview --restart")).toEqual({ build: "preview", adopt: false, label: "Refresh preview" });
  });

  it("carries --adopt, in either order", () => {
    expect(buildChangeOf("scripts/preview --adopt --restart")?.adopt).toBe(true);
    expect(buildChangeOf("  ./scripts/restart --main --adopt ")).toEqual({ build: "main", adopt: true, label: "Update to main" });
  });

  it("reads a script named by its full path", () => {
    expect(buildChangeOf("/Users/x/armada/scripts/preview --restart")?.build).toBe("preview");
  });

  it("leaves any other command a shell run", () => {
    for (const command of [
      "scripts/preview",
      "scripts/preview --restart --dry-run",
      "scripts/restart",
      "scripts/restart --main && echo done",
      "scripts/previewer --restart",
      "echo scripts/preview --restart",
      "kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt",
      "/land 2109 2118",
    ]) {
      expect(buildChangeOf(command), command).toBeUndefined();
    }
  });
});

describe("a waiting item's act read as the build switch", () => {
  it("reads a run of a restart, whichever script", () => {
    expect(restartOf({ kind: "run", target: "scripts/preview --restart" })?.label).toBe("Refresh preview");
    expect(restartOf({ kind: "run", target: "scripts/restart --main" })).toEqual({ build: "main", adopt: false, label: "Update to main" });
    expect(restartOf({ kind: "run", target: "/abs/path/scripts/preview --restart --adopt" })).toEqual({ build: "preview", adopt: true, label: "Refresh preview" });
  });

  it("leaves any other run, any other act and no act alone", () => {
    expect(restartOf({ kind: "run", target: "scripts/restart" })).toBeUndefined();
    expect(restartOf({ kind: "run", target: "scripts/preview --restart --dry-run" })).toBeUndefined();
    expect(restartOf({ kind: "run", target: "echo scripts/preview --restart" })).toBeUndefined();
    expect(restartOf({ kind: "run", target: "kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt" })).toBeUndefined();
    expect(restartOf({ kind: "run", target: "/land 2109" })).toBeUndefined();
    expect(restartOf({ kind: "walk", target: "scripts/preview --restart" })).toBeUndefined();
    expect(restartOf(undefined)).toBeUndefined();
  });
});
