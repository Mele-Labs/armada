// A restart handed to the person as a command, read as the Fleet panel's build switch.

import { describe, expect, it } from "vitest";

import { buildChangeOf } from "./build-change";

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
    ]) {
      expect(buildChangeOf(command), command).toBeUndefined();
    }
  });
});
