import { afterEach, describe, expect, test, vi } from "vitest";

import { attachPr } from "./claims";

const session = { id: "s10" } as const;

function fleetClaiming(answer: unknown) {
  const route = vi.fn(async () => answer);
  vi.stubGlobal("window", { armada: { claimPullRequest: route } });
  return route;
}

afterEach(() => vi.unstubAllGlobals());

describe("attaching a pull request nobody holds", () => {
  test("claims it for the Session by number, and says it was attached", async () => {
    const route = fleetClaiming({ ok: true, value: { number: 1823, branch: "b", url: "u", holder_kind: "session", holder_id: "s10" } });
    expect(await attachPr(1823, session)).toEqual({ attached: true });
    expect(route.mock.calls).toEqual([[{ number: 1823, session_id: "s10" }]]);
  });

  test("is refused in Fleet's words, which name the holder", async () => {
    fleetClaiming({
      ok: false,
      outcome: { ok: false, why: "refused", error: { code: "fleet.pull_request_not_claimable", message: "pull request #1823 is held by session Armada Pocket", run_id: "", fields: {}, chain: [] } },
    });
    expect(await attachPr(1823, session)).toEqual({ attached: false, said: "pull request #1823 is held by session Armada Pocket" });
  });
});
