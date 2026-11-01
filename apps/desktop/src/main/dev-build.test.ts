import { describe, expect, it } from "vitest";

import { isDevBuild } from "./dev-build";

describe("isDevBuild", () => {
  it("is a dev build when Electron says it is not packaged", () => {
    expect(isDevBuild(false, true)).toBe(true);
    expect(isDevBuild(false, undefined)).toBe(true);
  });

  // The Armada-named Electron copy: packaged by its executable's name, started with the source folder.
  it("is a dev build when the renamed copy runs the source through the default app", () => {
    expect(isDevBuild(true, true)).toBe(true);
  });

  it("is not a dev build when packaged and started on its own", () => {
    expect(isDevBuild(true, undefined)).toBe(false);
    expect(isDevBuild(true, false)).toBe(false);
  });
});
