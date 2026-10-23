// What `#` reads on a real Fleet, over the shapes Fleet answers: `list_issues`, the merge line's hub, and
// `get_pull_request` and `get_issue` for a number that is neither open.

import { describe, expect, it, vi } from "vitest";

import type { HubPullRequest, IssueList, MergeLine, PullRequestState, RepositorySummary, TrackedIssue } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import { FleetNumbered, STALE_MS } from "./numbered-fleet";

const ARMADA = "01M2ARMADA";
const SHOP = "01M2SHOP";
const REPOSITORIES = [
  { root: "/repos/armada", records_root: "/records/armada", manifest: { id: ARMADA } },
  { root: "/repos/shop", records_root: "/records/shop", manifest: { id: SHOP } },
] as unknown as RepositorySummary[];

const issue = (number: number, title: string): TrackedIssue => ({
  reference: `#${number}`,
  title,
  url: `https://forge.invalid/issues/${number}`,
  labels: [],
  assignees: [],
  updated_at: "2026-10-11T11:40:00Z",
  excerpt: "",
  linked_pull_request: false,
  state: "open",
  fields: [],
  holders: [],
});
const pull = (number: number, title: string, branch: string): HubPullRequest => ({ number, title, branch, url: `https://forge.invalid/pull/${number}` });
const line = (root: string, pulls: HubPullRequest[]): MergeLine => ({ root, line: [], off: [], landed: [], sent_back: [], hub: { pull_requests: pulls } });

const state = (lines: MergeLine[] | null) => ({ holds: { repositories: REPOSITORIES }, mergeLines: lines === null ? null : { lines } }) as never;

const PULLS = [pull(2219, "Pin the Bridge store clock", "fix/store-clock"), pull(2216, "Fold the two notification routes", "notify/one-route")];
const SHOP_PULLS = [pull(418, "Round cart totals once", "fix/cart-rounding")];

/** A Fleet that answers by repository, counting each ask. */
function fleet(answers: { issues?: Record<string, TrackedIssue[]>; pulls?: Record<string, PullRequestState>; found?: Record<string, { reference: string; title: string; url: string; state: "open" | "closed" }> } = {}) {
  const listIssues = vi.fn(async (manifestId: string) => ({
    ok: true as const,
    value: { manifest_id: manifestId, fetched_at: "2026-10-12T10:00:00Z", issues: answers.issues?.[manifestId] ?? [] } satisfies IssueList,
  }));
  const getPullRequest = vi.fn(async (manifestId: string, number: number) => {
    const held = answers.pulls?.[`${manifestId}#${number}`];
    return held === undefined
      ? { ok: false as const, outcome: { ok: false as const, why: "transport" as const, detail: "no such pull request", fault: { method: "GET" as const, path: "/pull_requests", why: "unreachable" as const } } }
      : { ok: true as const, value: held };
  });
  const getIssue = vi.fn(async (manifestId: string, reference: string) => {
    const held = answers.found?.[`${manifestId}${reference}`];
    return held === undefined
      ? { ok: false as const, outcome: { ok: false as const, why: "transport" as const, detail: "no such issue", fault: { method: "GET" as const, path: "/issues", why: "unreachable" as const } } }
      : { ok: true as const, value: held };
  });
  return { api: { listIssues, getPullRequest, getIssue } as unknown as Pick<BridgeApi, "listIssues" | "getIssue" | "getPullRequest">, listIssues, getPullRequest, getIssue };
}

function store(answers?: Parameters<typeof fleet>[0], now: () => number = () => 0) {
  const served = fleet(answers);
  const numbered = new FleetNumbered({ ...served.api, state: async () => state(null), subscribe: () => () => undefined } as never, now);
  return { ...served, numbered, source: numbered.source };
}

/** Lets the read that `open` started land. */
const landed = () => new Promise((done) => setTimeout(done, 0));

describe("the open ones", () => {
  it("are the repository's open issues and the merge line's open pull requests, tagged as `#` tags them", async () => {
    const held = store({ issues: { [ARMADA]: [issue(2214, "Sessions list drops a quiet Session"), issue(2212, "Helm answers about the wrong repository")] } });
    held.numbered.take(state([line("/repos/armada", PULLS), line("/repos/shop", SHOP_PULLS)]));

    // The merge line is held already; the issues are asked for the first time here.
    expect(held.source.open(ARMADA).map((one) => one.tag.kind)).toEqual(["pull_request", "pull_request"]);
    await landed();
    const hits = held.source.open(ARMADA);
    expect(hits.map((one) => [one.tag.kind, one.tag.id, one.tag.title])).toEqual([
      ["issue", "2214", "#2214 Sessions list drops a quiet Session"],
      ["issue", "2212", "#2212 Helm answers about the wrong repository"],
      ["pull_request", "2219", "#2219 Pin the Bridge store clock"],
      ["pull_request", "2216", "#2216 Fold the two notification routes"],
    ]);
    // The forge's address goes with each, for the wire.
    expect(hits.map((one) => one.tag.url)).toEqual([
      "https://forge.invalid/issues/2214",
      "https://forge.invalid/issues/2212",
      "https://forge.invalid/pull/2219",
      "https://forge.invalid/pull/2216",
    ]);
    expect(hits.every((one) => one.standing === undefined)).toBe(true);
  });

  it("are each repository's own, joined to its merge line by the repository's root", async () => {
    const held = store({ issues: { [SHOP]: [issue(412, "The log reader drops the last line")] } });
    held.numbered.take(state([line("/repos/armada", PULLS), line("/repos/shop", SHOP_PULLS)]));

    held.source.open(SHOP);
    await landed();
    expect(held.source.open(SHOP).map((one) => one.tag.id)).toEqual(["412", "418"]);
    expect(held.listIssues).toHaveBeenCalledTimes(1);
    expect(held.listIssues).toHaveBeenCalledWith(SHOP, false);
  });

  it("are the pull requests alone before the issues arrive, and none where nothing names the repository", () => {
    const held = store();
    held.numbered.take(state([line("/repos/armada", PULLS)]));
    expect(held.source.open(ARMADA).map((one) => one.tag.id)).toEqual(["2219", "2216"]);
    expect(held.source.open("01M2NOTSERVED")).toEqual([]);
    expect(held.source.open(undefined)).toEqual([]);
  });

  it("read the issues once, until they are stale, and again when `#` asks after that", async () => {
    let now = 0;
    const held = store({ issues: { [ARMADA]: [issue(2214, "Sessions list drops a quiet Session")] } }, () => now);
    held.source.open(ARMADA);
    held.source.open(ARMADA);
    await landed();
    held.source.open(ARMADA);
    expect(held.listIssues).toHaveBeenCalledTimes(1);

    now = STALE_MS + 1;
    held.source.open(ARMADA);
    held.source.open(ARMADA);
    expect(held.listIssues).toHaveBeenCalledTimes(2);
  });

  it("say so when the issues arrive and when a repository's pull requests change, and keep the same list otherwise", async () => {
    const held = store({ issues: { [ARMADA]: [issue(2214, "Sessions list drops a quiet Session")] } });
    held.numbered.take(state([line("/repos/armada", PULLS)]));
    const told = vi.fn();
    held.source.subscribe!(told);
    const before = held.source.version!();

    held.source.open(ARMADA);
    await landed();
    expect(told).toHaveBeenCalledTimes(1);
    expect(held.source.version!()).toBeGreaterThan(before);

    // The merge line moves but this repository's pull requests are the same ones.
    const list = held.source.open(ARMADA);
    held.numbered.take(state([line("/repos/armada", PULLS), line("/repos/shop", SHOP_PULLS)]));
    expect(told).toHaveBeenCalledTimes(1);
    expect(held.source.open(ARMADA)).toBe(list);

    held.numbered.take(state([line("/repos/armada", [PULLS[0]!]), line("/repos/shop", SHOP_PULLS)]));
    expect(told).toHaveBeenCalledTimes(2);
    expect(held.source.open(ARMADA).map((one) => one.tag.id)).toEqual(["2214", "2219"]);
  });

  it("keep what was read when a later read fails", async () => {
    let now = 0;
    const held = store({ issues: { [ARMADA]: [issue(2214, "Sessions list drops a quiet Session")] } }, () => now);
    held.source.open(ARMADA);
    await landed();
    held.listIssues.mockResolvedValueOnce({ ok: false, outcome: { ok: false, why: "not_connected" } } as never);
    now = STALE_MS + 1;
    held.source.open(ARMADA);
    await landed();
    expect(held.source.open(ARMADA).map((one) => one.tag.id)).toEqual(["2214"]);
  });
});

const done = (number: number, title: string, state: PullRequestState["state"]): PullRequestState => ({
  manifest_id: ARMADA,
  number,
  state,
  auto_merge: false,
  checks: { state: "passed" },
  title,
  branch: `merged/${number}`,
  address: `https://forge.invalid/pull/${number}`,
});

describe("a number that is not open", () => {
  it("is a pull request first, whatever its standing, and says where it stands", async () => {
    const held = store({ pulls: { [`${ARMADA}#2207`]: done(2207, "Stop the Drone count drifting", "merged"), [`${ARMADA}#2190`]: done(2190, "Drop the retired lock watchdog", "closed"), [`${ARMADA}#220`]: done(220, "Mind the gap", "draft") } });

    expect(await held.source.find(ARMADA, 2207)).toEqual({
      tag: { kind: "pull_request", id: "2207", title: "#2207 Stop the Drone count drifting", url: "https://forge.invalid/pull/2207" },
      standing: "Merged",
    });
    expect(await held.source.find(ARMADA, 2190)).toMatchObject({ tag: { kind: "pull_request", id: "2190" }, standing: "Closed" });
    // A draft is open, so it carries no standing.
    const draft = await held.source.find(ARMADA, 220);
    expect(draft?.tag).toMatchObject({ kind: "pull_request", id: "220" });
    expect(draft?.standing).toBeUndefined();
    expect(held.getIssue).not.toHaveBeenCalled();
  });

  it("is an issue where the forge has no pull request of that number, closed or open", async () => {
    const held = store({
      found: {
        [`${ARMADA}#61`]: { reference: "#61", title: "Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61", state: "closed" },
        [`${ARMADA}#62`]: { reference: "#62", title: "Doctor lists a Fleet twice", url: "https://forge.invalid/issues/62", state: "open" },
      },
    });

    expect(await held.source.find(ARMADA, 61)).toEqual({
      tag: { kind: "issue", id: "61", title: "#61 Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61" },
      standing: "Closed",
    });
    const open = await held.source.find(ARMADA, 62);
    expect(open?.tag).toMatchObject({ kind: "issue", id: "62" });
    expect(open?.standing).toBeUndefined();
    expect(held.getPullRequest).toHaveBeenCalledTimes(2);
  });

  it("is nothing where it is neither, or where no repository is named, or where Fleet cannot be asked", async () => {
    const held = store();
    expect(await held.source.find(ARMADA, 9999)).toBeUndefined();
    expect(await held.source.find(undefined, 61)).toBeUndefined();

    held.getPullRequest.mockRejectedValueOnce(new Error("the connection closed"));
    held.getIssue.mockRejectedValueOnce(new Error("the connection closed"));
    expect(await held.source.find(ARMADA, 61)).toBeUndefined();
  });
});
