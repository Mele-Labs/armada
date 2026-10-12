import { describe, expect, it, vi } from "vitest";

import { CHANNELS } from "../../shared/bridge";
import { follow, followedBy } from "./follow";

describe("which walk windows move Bridge", () => {
  it("a Job's moves it to that Job", () => {
    expect(followedBy({ job: { id: "job-7", handle: "fix-login" } })).toBe("job-7");
  });

  it("a Studio's and a Session's move nothing", () => {
    expect(followedBy({ studio: { id: "studio-1", name: null } })).toBeNull();
    expect(followedBy({ session: { id: "session-1", title: "Walk" } })).toBeNull();
  });
});

describe("a Job's walk window taking focus", () => {
  it("watches its Job and tells Bridge's window to open it, raising nothing", () => {
    const watch = vi.fn();
    const send = vi.fn();
    follow("job-7", { watch, bridge: () => ({ send }) });
    expect(watch).toHaveBeenCalledWith("job-7");
    expect(send).toHaveBeenCalledWith(CHANNELS.summoned, { jobId: "job-7", follow: true });
  });

  it("still watches it with no Bridge window open, so the panel has the Job to draw", () => {
    const watch = vi.fn();
    follow("job-7", { watch, bridge: () => null });
    expect(watch).toHaveBeenCalledWith("job-7");
  });
});
