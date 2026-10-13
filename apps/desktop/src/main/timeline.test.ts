import { describe, expect, it } from "vitest";

import { timelinePath } from "./timeline";
import { windowOf } from "./timeline-channels";

describe("the window main asks Fleet for", () => {
  it("is the bare route with nothing asked, and carries only what was", () => {
    expect(timelinePath({})).toBe("/timeline");
    expect(timelinePath({ since: "2026-10-13T08:00:00Z", limit: 500 })).toBe("/timeline?since=2026-10-13T08%3A00%3A00Z&limit=500");
    expect(timelinePath({ until: "2026-10-13T09:00:00Z" })).toBe("/timeline?until=2026-10-13T09%3A00%3A00Z");
  });

  it("takes nothing from the renderer but a string instant and a whole positive limit", () => {
    expect(windowOf(undefined)).toEqual({});
    expect(windowOf("since")).toEqual({});
    expect(windowOf({ since: 3, until: null, limit: "500" })).toEqual({});
    expect(windowOf({ since: "a", until: "b", limit: 0 })).toEqual({ since: "a", until: "b" });
    expect(windowOf({ limit: 2.5 })).toEqual({});
    expect(windowOf({ since: "a", limit: 40, port: 9 })).toEqual({ since: "a", limit: 40 });
  });
});
