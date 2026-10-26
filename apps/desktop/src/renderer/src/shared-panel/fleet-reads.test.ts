// The side panel's reads from Fleet, over a fake api: which repository a press names, what each kind of thing is read into, and
// what a Check follows. Nothing here draws; the panels draw what these answer.

import { expect, test, vi } from "vitest";
import type { FollowedLandLog, MergeLines, PullRequestPage, RepositorySummary, WorktreeSlot } from "@armada/protocol";

import { fleetPanelReads, repositoryOf } from "./fleet-reads";
import type { FleetPanelApi, FleetPanelState } from "./fleet-reads";
import type { PullRequestRead } from "./reads";

const repo = (id: string, name: string, root: string): RepositorySummary => ({ root, records_root: `/records${root}`, manifest: { id, repository: name } as never });
const ARMADA = repo("m-armada", "mele-labs/armada", "/code/armada");
const POCKET = repo("m-pocket", "mele-labs/pocket", "/code/pocket");

test("a press names its repository by Manifest, checkout, forge name or address, and the one there is when none of them does", () => {
  const both = [ARMADA, POCKET];
  expect(repositoryOf(both, { manifestId: "m-pocket" })).toBe(POCKET);
  expect(repositoryOf(both, { root: "/code/armada" })).toBe(ARMADA);
  expect(repositoryOf(both, { repository: "Mele-Labs/Pocket" })).toBe(POCKET);
  expect(repositoryOf(both, { address: "https://github.com/mele-labs/pocket/pull/9" })).toBe(POCKET);
  expect(repositoryOf(both, { address: "https://github.com/mele-labs/pocket/issues/9" })).toBe(POCKET);
  // The Manifest may call the repository by its bare name.
  expect(repositoryOf([repo("m-bare", "armada", "/code/bare"), POCKET], { repository: "mele-labs/armada" })?.manifest?.id).toBe("m-bare");
  // The Manifest wins over a forge name that points elsewhere; nothing named is no repository when there are two.
  expect(repositoryOf(both, { manifestId: "m-armada", repository: "mele-labs/pocket" })).toBe(ARMADA);
  expect(repositoryOf(both, {})).toBeUndefined();
  expect(repositoryOf(both, { address: "https://forge.invalid/pull/9" })).toBeUndefined();
  expect(repositoryOf([ARMADA], {})).toBe(ARMADA);
  // A repository with no Manifest is not one a read can be made of.
  expect(repositoryOf([{ root: "/code/bare", records_root: "/r" }], {})).toBeUndefined();
});

const NONE_FOLLOWED = { state: "none" } as const;
const state = (over: Partial<FleetPanelState> = {}): FleetPanelState =>
  ({
    holds: { repositories: [ARMADA, POCKET] },
    mergeLines: null,
    held: { state: "unread" },
    checkoutRunSheet: { state: "none" },
    landFollowed: NONE_FOLLOWED,
    ...over,
  }) as unknown as FleetPanelState;

/** An api whose every read is a failure, so a test names the reads it expects by giving them. */
const api = (given: Partial<Record<keyof FleetPanelApi, unknown>> = {}): FleetPanelApi => {
  const refused = async () => ({ ok: false, outcome: { kind: "refused" } });
  return {
    readReviewPage: refused,
    readReviewPatches: refused,
    writeReview: refused,
    getPullRequest: refused,
    pressRepositoryPullRequest: refused,
    getIssue: refused,
    getSlotHistory: refused,
    listCheckoutRuns: async () => ({ ok: true, runs: { runs: [] } }),
    getCheckoutRunOutput: refused,
    followLandCheck: vi.fn(async () => undefined),
    readHeld: vi.fn(async () => undefined),
    ...given,
  } as unknown as FleetPanelApi;
};

const page = (over: Partial<PullRequestPage> = {}): PullRequestPage => ({
  manifest_id: "m-armada",
  number: 7,
  base: "main",
  additions: 3,
  deletions: 1,
  mergeable: "mergeable",
  review: "none",
  approvals: 0,
  conversation: [],
  checks: [],
  files: [],
  commits: [],
  ...over,
});

const PR = { kind: "pr", number: 7, address: "https://github.com/mele-labs/armada/pull/7" } as const;

test("a pull request is the forge's standing as facts and its page, in the repository the press names", async () => {
  const getPullRequest = vi.fn(async () => ({
    ok: true,
    value: { manifest_id: "m-armada", number: 7, state: "open", auto_merge: true, queued: true, checks: { state: "failed", failing: ["lint", "test"] }, title: "Pin the clock", branch: "fix/clock", address: PR.address, body: "Closes #3." },
  }));
  const readReviewPage = vi.fn(async () => ({ ok: true, value: page({ author: "marta" }) }));
  const read = await fleetPanelReads(api({ getPullRequest, readReviewPage }), () => state()).pullRequest(PR);
  expect(getPullRequest).toHaveBeenCalledWith("m-armada", 7);
  expect(readReviewPage).toHaveBeenCalledWith("m-armada", 7);
  expect(read).toMatchObject({
    repository: "mele-labs/armada",
    root: "/code/armada",
    facts: { kind: "pull_request", number: 7, title: "Pin the clock", branch: "fix/clock", address: PR.address, state: "open", auto: true, queued: true, body: "Closes #3.", checks: { state: "failed", failing: "lint, test" } },
    page: { author: "marta" },
  });
});

test("a pull request's patches are read in one call, and a refusal is Fleet's words", async () => {
  const files = [{ manifest_id: "m-armada", number: 7, path: "src/clock.rs", patch: "@@ -1 +1 @@\n-a\n+b" }];
  const readReviewPatches = vi.fn(async () => ({ ok: true, value: { files } }));
  expect(await fleetPanelReads(api({ readReviewPatches }), () => state()).pullRequestPatches!(PR)).toEqual({ ok: true, files });
  expect(readReviewPatches).toHaveBeenCalledWith("m-armada", 7);
  const refused = async () => ({ ok: false, outcome: { ok: false, why: "refused", error: { message: "The forge refused the read." } } });
  expect(await fleetPanelReads(api({ readReviewPatches: refused }), () => state()).pullRequestPatches!(PR)).toEqual({ ok: false, said: "The forge refused the read." });
});

test("a pull request the forge's standing cannot be read for is drawn from what the press knew, and one with no page is not drawn", async () => {
  const readReviewPage = vi.fn(async () => ({ ok: true, value: page() }));
  const read = await fleetPanelReads(api({ readReviewPage }), () => state()).pullRequest({ ...PR, title: "From the row", branch: "fix/row" });
  expect((read as PullRequestRead).facts).toMatchObject({ number: 7, title: "From the row", branch: "fix/row", address: PR.address, state: "open", auto: false, checks: { state: "pending" } });
  expect((read as PullRequestRead).page.number).toBe(7);
  // The page is the one thing the panel cannot do without; nor can it read in a repository nobody names.
  expect(await fleetPanelReads(api(), () => state()).pullRequest(PR)).toBeUndefined();
  expect(await fleetPanelReads(api({ readReviewPage }), () => state()).pullRequest({ kind: "pr", number: 7, address: "https://forge.invalid/pull/7" })).toBeUndefined();
});

test("an issue is read whole, with its linked pull requests in the state each is in", async () => {
  const getIssue = vi.fn(async () => ({
    ok: true,
    value: {
      reference: "#2214",
      title: "List drops a quiet Session",
      url: "https://github.com/mele-labs/armada/issues/2214",
      state: "closed",
      body: "It is gone after a reload.",
      author: "nick",
      opened_at: "2026-10-01T10:00:00Z",
      labels: ["bug"],
      assignees: ["marta"],
      comments: [{ author: "marta", at: "2026-10-02T10:00:00Z", body: "Reproduced." }],
      linked_pull_requests: [
        { number: 1892, title: "Add the marker", url: "https://github.com/mele-labs/armada/pull/1892", state: "merged" },
        { number: 1893, title: "Try a cache", url: "https://github.com/mele-labs/armada/pull/1893", state: "closed" },
        { number: 1894, title: "Keep it", url: "https://github.com/mele-labs/armada/pull/1894", state: "open" },
      ],
    },
  }));
  const read = await fleetPanelReads(api({ getIssue }), () => state()).issue({ kind: "issue", number: "2214", manifestId: "m-armada" });
  expect(getIssue).toHaveBeenCalledWith("m-armada", "#2214");
  expect(read).toEqual({
    number: "2214",
    title: "List drops a quiet Session",
    repository: "mele-labs/armada",
    address: "https://github.com/mele-labs/armada/issues/2214",
    state: "closed",
    author: "nick",
    openedAt: "2026-10-01T10:00:00Z",
    labels: ["bug"],
    assignees: ["marta"],
    body: "It is gone after a reload.",
    comments: [{ author: "marta", at: "2026-10-02T10:00:00Z", body: "Reproduced." }],
    linked: [
      { number: 1892, title: "Add the marker", address: "https://github.com/mele-labs/armada/pull/1892", state: "merged" },
      { number: 1893, title: "Try a cache", address: "https://github.com/mele-labs/armada/pull/1893", state: "closed" },
      { number: 1894, title: "Keep it", address: "https://github.com/mele-labs/armada/pull/1894", state: "open" },
    ],
  });
  expect(await fleetPanelReads(api(), () => state()).issue({ kind: "issue", number: "1", manifestId: "m-armada" })).toBeUndefined();
});

// ---- A Check -------------------------------------------------------------------------------------------------

const lines = (): MergeLines => ({
  lines: [
    {
      root: "/code/armada",
      line: [{ branch: "feat/a", state: "gating", checks: [{ name: "test", state: "running", started_at: "2026-10-10T10:00:00Z" }] }],
      off: [],
      landed: [],
      sent_back: [{ branch: "feat/b", state: "red", checks: [{ name: "test", state: "failed" }] }],
      hub: { pull_requests: [{ number: 7, title: "A", branch: "feat/a", url: "https://github.com/mele-labs/armada/pull/7" }] },
    },
  ],
});
const LAND = { kind: "check", id: "land:/code/armada:feat/a:test" } as const;

test("a merge line Check follows its log once, and reads the lines once the log followed is its own", async () => {
  const api1 = api();
  const first = await fleetPanelReads(api1, () => state({ mergeLines: lines() })).check(LAND);
  expect(api1.followLandCheck).toHaveBeenCalledTimes(1);
  expect(api1.followLandCheck).toHaveBeenCalledWith({ root: "/code/armada", branch: "feat/a", check: "test" });
  expect(first).toMatchObject({ name: "test", repository: "mele-labs/armada", state: "running", log: { run: "feat/a", lines: [] } });

  const followed: FollowedLandLog = { state: "following", root: "/code/armada", branch: "feat/a", check: "test", fromLine: 1, lines: ["one", "two"] };
  const api2 = api();
  const second = await fleetPanelReads(api2, () => state({ mergeLines: lines(), landFollowed: followed })).check(LAND);
  expect(api2.followLandCheck).not.toHaveBeenCalled();
  expect(second?.log.lines).toEqual(["one", "two"]);

  // Another Check's log being followed is not this one's.
  const api3 = api();
  const other = await fleetPanelReads(api3, () => state({ mergeLines: lines(), landFollowed: { ...followed, check: "build" } })).check(LAND);
  expect(api3.followLandCheck).toHaveBeenCalledTimes(1);
  expect(other?.log.lines).toEqual([]);
});

test("a Check lists the runs of its own name on the line, the pull request each is on named", async () => {
  const read = await fleetPanelReads(api(), () => state({ mergeLines: lines() })).check(LAND);
  const run = (on: string) => read?.runs.find((one) => one.on === on);
  expect(read?.runs).toHaveLength(2);
  expect(run("feat/a")).toMatchObject({ state: "running", pull: { number: 7, address: "https://github.com/mele-labs/armada/pull/7" } });
  expect(run("feat/b")).toMatchObject({ state: "failed" });
  expect(run("feat/b")).not.toHaveProperty("pull");
});

test("a run on main is no row of the line, and its log is followed from the names the press gave", async () => {
  const mine = api();
  const item = { kind: "check", id: "land:/code/armada:main:desktop_test", land: { root: "/code/armada", branch: "main", check: "desktop_test" } } as const;
  const read = await fleetPanelReads(mine, () => state({ mergeLines: lines() })).check(item);
  expect(mine.followLandCheck).toHaveBeenCalledWith(item.land);
  expect(read).toMatchObject({ name: "desktop_test", repository: "mele-labs/armada", state: "failed", log: { run: "main" } });
  // With nothing naming it, an unknown Check is not drawn.
  expect(await fleetPanelReads(api(), () => state({ mergeLines: lines() })).check({ kind: "check", id: "land:/code/armada:main:other" })).toBeUndefined();
});

test("a row of the forge's checks on a pull request reads from the row, and asks Fleet nothing", async () => {
  const none = new Proxy({}, { get: () => () => Promise.reject(new Error("asked Fleet")) }) as unknown as FleetPanelApi;
  const read = await fleetPanelReads(none, () => undefined).check({ kind: "check", id: "https://forge/checks/1", name: "test · bridge", branch: "fix/x", row: { state: "failed", seconds: 192 } });
  expect(read).toEqual({
    name: "test · bridge",
    repository: "",
    state: "failed",
    runs: [{ on: "fix/x", state: "failed", seconds: 192, ago: "" }],
    log: { run: "fix/x", lines: [] },
  });
});

// ---- A bay ----------------------------------------------------------------------------------------------------

const HOUR = 3_600_000;
const at = (ago: number) => new Date(Date.now() - ago).toISOString();
const slot = (over: Partial<WorktreeSlot> = {}): WorktreeSlot => ({
  manifest_id: "m-armada",
  slot: 3,
  path: "/code/armada/.armada/slots/slot-3",
  base: "main",
  warm: true,
  behind: 2,
  held: { state: "session", holder: "claude (pid 1)", session_title: "Fix the wait" } as never,
  branch: "fix/the-wait",
  since: at(HOUR),
  ...over,
});

test("a bay is read from the pool's slot, with its history paired into spans, the newest first", async () => {
  const mine = api({
    getSlotHistory: async () => ({
      ok: true,
      value: {
        manifest_id: "m-armada",
        slot: 3,
        entries: [
          { kind: "leased", at: at(HOUR), holder: { kind: "session", id: "s-now", title: "Fix the wait" }, branch: "fix/the-wait" },
          { kind: "released", at: at(3 * HOUR), holder: { kind: "session", id: "s-before", title: "Cache key" } },
          { kind: "leased", at: at(5 * HOUR), holder: { kind: "session", id: "s-before", title: "Cache key" }, branch: "fix/cache-key" },
          { kind: "checkout", at: at(4 * HOUR), sha: "abc1234", to: "main" },
        ],
      },
    }),
  });
  const held = { state: "read", held: { slots: [slot(), slot({ slot: 4 })] } };
  const read = await fleetPanelReads(mine, () => state({ held: held as never })).bay({ kind: "bay", manifestId: "m-armada", slot: 3 });
  expect(read).toMatchObject({ name: "slot-3", repository: "mele-labs/armada", branch: "fix/the-wait", base: "main", behind: 2, warm: true, path: "/code/armada/.armada/slots/slot-3" });
  expect(read?.slot?.slot).toBe(3);
  // A lease ends at the release above it: Cache key held for two of the hours between its lease and the one after.
  expect(read?.history).toEqual([
    { who: "Fix the wait", span: "1h 00m", branch: "fix/the-wait" },
    { who: "Cache key", span: "2h 00m", branch: "fix/cache-key" },
  ]);
  // The pool was already read, so it was not asked for.
  expect(mine.readHeld).not.toHaveBeenCalled();
});

test("a bay the pool does not hold is not drawn", async () => {
  vi.useFakeTimers();
  try {
    const mine = api();
    // The pool is not read yet, so Fleet is asked to serve it; it never holds this slot.
    const pending = fleetPanelReads(mine, () => state()).bay({ kind: "bay", manifestId: "m-armada", slot: 3 });
    await vi.advanceTimersByTimeAsync(3500);
    expect(await pending).toBeUndefined();
    expect(mine.readHeld).toHaveBeenCalledWith(true);
  } finally {
    vi.useRealTimers();
  }
});

test("a page Fleet refused is its words; a read that failed some other way answers nothing", async () => {
  const readReviewPage = async () => ({ ok: false, outcome: { ok: false, why: "refused", error: { message: "GitHub refused access. Run gh auth refresh." } } });
  expect(await fleetPanelReads(api({ readReviewPage }), () => state()).pullRequest(PR)).toEqual({ refused: "GitHub refused access. Run gh auth refresh.", root: "/code/armada" });
  const notConnected = async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } });
  expect(await fleetPanelReads(api({ readReviewPage: notConnected }), () => state()).pullRequest(PR)).toBeUndefined();
});
