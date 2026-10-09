import { describe, expect, test } from "vitest";

import { mergeLineViews } from "@armada/screens";
import type { MergeLines } from "@armada/protocol";

import { chipsOf, landingChips, pullBlocks, shortBranch } from "./horizon";

const pull = (number: number, branch: string, more: Record<string, unknown> = {}) => ({ number, title: `Title ${number}`, branch, url: `https://git.example/pull/${number}`, ...more });

const lines = (pull_requests: unknown[]): MergeLines =>
  ({ lines: [{ root: "/armada", line: [], off: [], landed: [], sent_back: [], hub: { pull_requests } }] }) as unknown as MergeLines;

/** What the strip reads: the same fold the Merge line surface draws, then the blocks off it. */
function blocksOf(served: MergeLines, sessions: Parameters<typeof pullBlocks>[3] = []) {
  const views = mergeLineViews(served, null, [], []);
  return pullBlocks(views[0]!, served, views, sessions);
}

describe("the horizon's pull requests", () => {
  const served = lines([
    pull(5, "chore/bump", { ci: "passed" }),
    pull(4, "fix/open", { ci: "failed" }),
    pull(3, "feat/second", { ci: "passed", queue: { state: "queued", position: 2 } }),
    pull(2, "feat/first", { ci: "passed", queue: { state: "awaiting_checks", position: 1 } }),
    pull(1, "docs/waiting", { ci: "running", queue: { state: "waiting_for_ci" } }),
    pull(6, "wip/nothing-ran"),
  ]);

  test("the queue comes first by position, then those waiting to join it, then the rest as listed", () => {
    expect(blocksOf(served).map((one) => one.number)).toEqual([2, 3, 1, 5, 4, 6]);
  });

  test("a pull request in the queue is queued, and one waiting to join it is not", () => {
    expect(blocksOf(served).map((one) => one.queued)).toEqual([true, true, false, false, false, false]);
  });

  test("the state is the queue's where there is one, else the ci's, and none where nothing ran", () => {
    expect(blocksOf(served).map((one) => one.state)).toEqual(["running", "queued", "running", "ready", "failing", "open"]);
    expect(blocksOf(served).map((one) => one.mark?.statusToken ?? null)).toEqual([
      "--status-running",
      "--status-not-started",
      "--status-running",
      "--status-completed-success",
      "--status-completed-failed",
      null,
    ]);
  });

  test("the hover gives title, branch, state and place", () => {
    expect(blocksOf(served)[0]!.tip).toBe("#2 Title 2 · feat/first · In the merge queue, running its checks, place 1");
  });

  test("the hover names the Job that opened it, else the Session that holds it", () => {
    const withJob = lines([pull(7, "fleet/pause", { job: { id: "j1", title: "Store a pause marker" } }), pull(8, "pocket/pwa")]);
    const holds = [{ id: "s13", title: "Armada Pocket", rows: [], attachments: [{ kind: "pull_request", number: 8, branch: "pocket/pwa" }] }] as unknown as Parameters<typeof pullBlocks>[3];
    const [first, second] = blocksOf(withJob, holds);
    expect(first!.tip).toContain("Job: Store a pause marker");
    expect(second!.tip).toContain("Session: Armada Pocket");
  });

  test("a line with no hub has none", () => {
    const bare = { lines: [{ root: "/armada", line: [], off: [], landed: [], sent_back: [] }] } as unknown as MergeLines;
    expect(blocksOf(bare)).toEqual([]);
  });
});

describe("the landings as chips", () => {
  const line = (rows: unknown[]): MergeLines =>
    ({ lines: [{ root: "/armada", line: rows, off: [], landed: [], sent_back: [], hub: { pull_requests: [pull(9, "feat/open", { ci: "passed" })] } }] }) as unknown as MergeLines;

  test("a branch is its last path segment, an agent's worktree its first four hex, at most 12 characters", () => {
    expect(shortBranch("fleet/gate-policy")).toBe("gate-policy");
    expect(shortBranch("fleet/read-in-cluster-membership")).toBe("read-in-clu…");
    expect(shortBranch("worktree-agent-aef3c24792026e2c3")).toBe("agent-aef3");
    expect(shortBranch("main")).toBe("main");
  });

  test("a stage is a state colour, and the hover gives the full branch and its stage", () => {
    const served = line([
      { place: 1, branch: "docs/wire-lock-signed", state: "gating", checks: [{ name: "build", state: "running" }] },
      { place: 2, branch: "worktree-agent-aef3c24792026e2c3", state: "preparing" },
      { place: 3, branch: "fleet/pulse-log-rows", state: "red" },
      { place: 4, branch: "fleet/helm-kills-processes", state: "waiting" },
    ]);
    const views = mergeLineViews(served, null, [], []);
    const chips = landingChips(views[0]!.line);
    expect(chips.map((one) => one.state)).toEqual(["running", "running", "failing", "queued"]);
    expect(chips.map((one) => one.label)).toEqual(["wire-lock-s…", "agent-aef3", "pulse-log-r…", "helm-kills-…"]);
    expect(chips[0]!.tip).toBe("docs/wire-lock-signed · Running Checks before landing");
    expect(chips.every((one) => one.icon !== null && one.queued)).toBe(true);
  });

  test("one row to main: the landings first, then the queue, then the open ones", () => {
    const served = {
      lines: [
        {
          root: "/armada",
          line: [{ place: 1, branch: "docs/a", state: "gating" }],
          off: [],
          landed: [],
          sent_back: [],
          hub: { pull_requests: [pull(5, "x/open", { ci: "passed" }), pull(3, "x/queued", { ci: "passed", queue: { state: "queued", position: 1 } })] },
        },
      ],
    } as unknown as MergeLines;
    const views = mergeLineViews(served, null, [], []);
    expect(chipsOf(views[0]!, served, views, []).map((one) => one.label)).toEqual(["a", "#3", "#5"]);
  });
});
