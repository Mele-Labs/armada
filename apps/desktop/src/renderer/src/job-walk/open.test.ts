// A native walk's row opens the native window and leaves the Cockpit where it is; the tab comes forward only when Fleet does not open it.

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

const bringWalkForward = vi.fn();
const askToOpenSession = vi.fn();
vi.mock("./store", () => ({
  bringWalkForward: (...args: unknown[]) => bringWalkForward(...args),
}));
vi.mock("../open-session", () => ({
  askToOpenSession: (...args: unknown[]) => askToOpenSession(...args),
}));

const { openJobWalk } = await import("./open");

const NATIVE = "armada://walk/jobs-list";

describe("openJobWalk", () => {
  beforeEach(() => {
    bringWalkForward.mockReset();
    askToOpenSession.mockReset();
  });
  afterEach(() => vi.unstubAllGlobals());

  test("a native walk opens its window and brings no tab forward", async () => {
    const openNative = vi.fn(async () => ({
      ok: true as const,
    }));
    vi.stubGlobal("window", {
      armada: {
        jobWalk: {
          openNative,
        },
      },
    });
    openJobWalk("job-1", NATIVE);
    await vi.waitFor(() => expect(openNative).toHaveBeenCalledWith("job-1", NATIVE));
    await Promise.resolve();
    expect(bringWalkForward).not.toHaveBeenCalled();
    expect(askToOpenSession).not.toHaveBeenCalled();
  });

  test("a native walk Fleet does not open brings its tab forward, where Open and the reason are", async () => {
    vi.stubGlobal("window", {
      armada: {
        jobWalk: {
          openNative: async () => ({
            ok: false as const,
            why: "unreachable" as const,
          }),
        },
      },
    });
    openJobWalk("job-1", NATIVE);
    await vi.waitFor(() => expect(bringWalkForward).toHaveBeenCalledWith("job-1", NATIVE));
    expect(askToOpenSession).toHaveBeenCalledWith("job-1");
  });

  test("a page's walk brings its tab forward", () => {
    vi.stubGlobal("window", {});
    openJobWalk("job-1", "http://localhost:5173/?walk=jobs");
    expect(bringWalkForward).toHaveBeenCalledWith("job-1", "http://localhost:5173/?walk=jobs");
    expect(askToOpenSession).toHaveBeenCalledWith("job-1");
  });
});
