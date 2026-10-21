// The side panel's reads from Fleet: the same calls the Reviews page, the bays and the Checks page make, so the panel says
// what they say. **Read at the moment a panel opens**, off the window's state as it stands then (`now`), so one set of reads
// serves the window for as long as it is up. A mock that brings fixtures of its own passes those instead.

import type { CheckoutRunRecord, MergeLines, PullRequestState, RepositorySummary } from "@armada/protocol";
import { agoOf } from "@armada/components";
import { checkEntriesOf } from "@armada/screens/src/manifest-checks";
import type { CheckEntry } from "@armada/screens/src/manifest-checks";
import { refusalWords } from "@armada/screens/src/refusal-words";
import { issueLabel, type SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import { bayViewOf, ledgerOf, slotKey, spanOf, shortOf } from "../cockpit/deck/bays";
import type { BayRead, CheckRead, IssueRead, PanelReads, PullRequestRead } from "./reads";
import type { PanelItem } from "./store";

type Of<K extends PanelItem["kind"]> = Extract<PanelItem, { kind: K }>;
type Held = Extract<SessionAttachment, { kind: "pull_request" }>;

export type FleetPanelApi = Pick<
  BridgeApi,
  "readReviewPage" | "readReviewPatches" | "writeReview" | "getPullRequest" | "pressRepositoryPullRequest" | "getIssue" | "getSlotHistory" | "listCheckoutRuns" | "getCheckoutRunOutput" | "followLandCheck" | "readHeld"
>;
export type FleetPanelState = Pick<BridgeState, "holds" | "mergeLines" | "held" | "checkoutRunSheet" | "landFollowed">;

const NONE: readonly never[] = [];

/** `/<owner>/<name>/pull/<n>` or `/issues/<n>`: the repository an address is on, as a Manifest names it. */
const repositoryOfAddress = (address: string | undefined): string | undefined => /^https?:\/\/[^/]+\/([^/]+\/[^/]+)\/(?:pull|issues)\//.exec(address ?? "")?.[1];

/** The repository a press names: by its checkout, by its forge name (`owner/name`, or the bare name a Manifest may call it), by its address, or the one there is. */
export function repositoryOf(repositories: readonly RepositorySummary[], at: { root?: string; manifestId?: string; repository?: string; address?: string }): RepositorySummary | undefined {
  const managed = repositories.filter((one) => one.manifest !== undefined);
  const named = at.repository ?? repositoryOfAddress(at.address);
  return (
    (at.manifestId === undefined ? undefined : managed.find((one) => one.manifest?.id === at.manifestId)) ??
    (at.root === undefined ? undefined : managed.find((one) => one.root === at.root)) ??
    (named === undefined ? undefined : managed.find((one) => [named.toLowerCase(), named.split("/").at(-1)!.toLowerCase()].includes(one.manifest?.repository?.toLowerCase() ?? ""))) ??
    (managed.length === 1 ? managed[0] : undefined)
  );
}

/** The forge's pull request as a Session's attachment carries it, so the panel draws the two the same. */
export function factsOf(state: PullRequestState): Held {
  const checks: Held["checks"] = state.checks.state === "failed" ? { state: "failed", failing: state.checks.failing.join(", ") } : { state: state.checks.state };
  return {
    kind: "pull_request",
    number: state.number,
    title: state.title,
    branch: state.branch,
    address: state.address,
    checks,
    // A closed pull request has no word of its own on a Session; it is drawn as the forge's open one was.
    state: state.state === "closed" ? "open" : state.state,
    auto: state.auto_merge,
    ...(state.queued === true ? { queued: true as const } : {}),
    ...(state.body === undefined ? {} : { body: state.body }),
  };
}

const RUN: Partial<Record<CheckEntry["status"], CheckRead["state"]>> = { passed: "passed", failed: "failed" };
const runStateOf = (entry: CheckEntry): CheckRead["state"] => RUN[entry.status] ?? "running";
const whereOf = (entry: CheckEntry): string => entry.land?.branch ?? entry.record?.workspace ?? "checkout";


/** The pull request a merge line branch is open as. */
const pullOfBranch = (lines: MergeLines | null, root: string | undefined, branch: string | undefined) => {
  if (root === undefined || branch === undefined) return undefined;
  const pr = lines?.lines.find((one) => one.root === root)?.hub?.pull_requests?.find((one) => one.branch === branch);
  return pr === undefined ? undefined : { number: pr.number, address: pr.url };
};

export function fleetPanelReads(api: FleetPanelApi, now: () => FleetPanelState | undefined): Omit<PanelReads, "lines" | "numbered"> {
  const repositories = () => now()?.holds.repositories ?? NONE;
  const manifestOfPr = (item: Of<"pr">) =>
    repositoryOf(repositories(), { ...(item.manifestId === undefined ? {} : { manifestId: item.manifestId }), ...(item.root === undefined ? {} : { root: item.root }), ...(item.repository === undefined ? {} : { repository: item.repository }), address: item.address });

  const pullRequest = async (item: Of<"pr">): Promise<PullRequestRead | undefined> => {
    const repo = manifestOfPr(item);
    const manifestId = repo?.manifest?.id;
    if (repo === undefined || manifestId === undefined) return undefined;
    const [standing, page] = await Promise.all([api.getPullRequest(manifestId, item.number).catch(() => undefined), api.readReviewPage(manifestId, item.number).catch(() => undefined)]);
    if (page?.ok !== true) return undefined;
    const facts: Held =
      standing?.ok === true
        ? factsOf(standing.value)
        : { kind: "pull_request", number: item.number, title: item.title ?? "", branch: item.branch ?? "", address: item.address, checks: { state: "pending" }, state: "open", auto: false };
    return { repository: repo.manifest?.repository ?? "", root: repo.root, facts, page: page.value };
  };

  const bay = async (item: Of<"bay">): Promise<BayRead | undefined> => {
    // Fleet serves the pool while something draws it. Asked here and not let go, since the Cockpit's own ask is the same switch.
    if (now()?.held.state !== "read") void api.readHeld(true);
    const slot = await until(now, (state) => (state.held.state === "read" ? (state.held.held.slots ?? []).find((one) => one.manifest_id === item.manifestId && one.slot === item.slot) : undefined));
    if (slot === undefined) return undefined;
    const view = bayViewOf(slot, slotKey(slot), ledgerOf(slot, []), undefined, Date.now());
    const history = await api.getSlotHistory(item.manifestId, item.slot).catch(() => undefined);
    const entries = history?.ok === true ? history.value.entries : [];
    // Newest first, as Fleet serves them: the release that ends a lease is the nearest one above it.
    const spans = entries.flatMap((one, at) => {
      if (one.kind !== "leased") return [];
      const freed = entries.slice(0, at).reverse().find((later) => later.kind === "released" && later.holder.id === one.holder.id);
      const ms = (freed === undefined ? Date.now() : Date.parse(freed.at)) - Date.parse(one.at);
      return [{ who: one.holder.title ?? shortOf(one.holder.id), span: Number.isNaN(ms) ? "" : spanOf(ms), ...(one.branch === undefined ? {} : { branch: one.branch }) }];
    });
    const state: BayRead["state"] = view.state === "leased" || view.state === "stranded" || view.state === "closed" ? view.state : "free";
    return {
      name: `slot-${slot.slot}`,
      repository: repositoryOf(repositories(), { manifestId: slot.manifest_id })?.manifest?.repository ?? "",
      state,
      word: view.said ?? view.word,
      ...(view.holder === undefined ? {} : { holder: { id: view.holder.id, title: view.holder.title ?? shortOf(view.holder.id) } }),
      ...(slot.branch === undefined ? {} : { branch: slot.branch }),
      ...(view.heldFor === undefined ? {} : { heldFor: spanOf(view.heldFor) }),
      base: slot.base,
      behind: slot.behind ?? 0,
      warm: slot.warm,
      path: slot.path,
      uncommitted: slot.stranded?.uncommitted ?? NONE,
      unpushed: (slot.stranded?.commits ?? []).filter((one) => one.home === "only_here").map((one) => `${one.sha.slice(0, 7)} ${one.subject}`),
      history: spans,
      slot,
    };
  };

  const check = async (item: Of<"check">): Promise<CheckRead | undefined> => {
    // A row of the forge's checks on a pull request: what the forge said of it, and its log is the forge's.
    if (item.row !== undefined) {
      return { name: item.name ?? item.id, repository: "", state: item.row.state, runs: [{ on: item.branch ?? "", state: item.row.state, ...(item.row.seconds === undefined ? {} : { seconds: item.row.seconds }), ago: "" }], log: { run: item.branch ?? "", lines: NONE } };
    }
    const state = now();
    if (state === undefined) return undefined;
    const listed = await api.listCheckoutRuns().catch(() => undefined);
    const records: readonly CheckoutRunRecord[] = listed?.ok === true ? listed.runs.runs : NONE;
    const sheet = state.checkoutRunSheet.state === "read" ? state.checkoutRunSheet.sheet : undefined;
    const entries = checkEntriesOf(sheet, [...records], state.mergeLines?.lines ?? []);
    const entry = entries.find((one) => one.id === item.id);
    // A run on main is no row of any line's Checks, but the press that opened it named the Check: its log is followed all the same.
    // The merge line opens only a failed run's log without a row.
    const land = entry?.land ?? item.land;
    if (land === undefined && entry === undefined) return undefined;
    const root = land?.root;
    const runs =
      entry === undefined
        ? [{ on: land!.branch, state: "failed" as const, ago: "" }]
        : entries
            .filter((one) => one.name === entry.name && one.land?.root === root)
            .map((one) => {
              const pull = pullOfBranch(state.mergeLines, one.land?.root, one.land?.branch);
              const seconds = one.record === undefined ? undefined : Math.round(one.record.duration_ms / 1000);
              return { on: whereOf(one), state: runStateOf(one), ...(seconds === undefined ? {} : { seconds }), ago: agoOf(one.startedAt ?? one.requestedAt) ?? "", ...(pull === undefined ? {} : { pull }) };
            });
    let lines: readonly string[] = NONE;
    if (land !== undefined) {
      const followed = state.landFollowed;
      const same = followed.state !== "none" && followed.root === land.root && followed.branch === land.branch && followed.check === land.check;
      if (!same) void api.followLandCheck(land);
      if (same && followed.state === "following") lines = followed.lines;
    } else if (entry?.runId !== undefined) {
      const output = await api.getCheckoutRunOutput(entry.runId).catch(() => undefined);
      if (output?.ok === true) lines = output.output.lines;
    }
    const repository = root === undefined ? undefined : repositoryOf(repositories(), { root })?.manifest?.repository;
    return {
      name: entry?.name ?? land!.check,
      repository: repository ?? "",
      state: entry === undefined ? "failed" : runStateOf(entry),
      runs,
      log: { run: entry === undefined ? land!.branch : whereOf(entry), lines },
    };
  };

  return {
    pullRequest,
    writePullRequest: async (item, write) => {
      const manifestId = manifestOfPr(item)?.manifest?.id;
      if (manifestId === undefined) return { ok: false, said: refusalWords({ ok: false, why: "no_manifest" } as never) };
      const answer = await api.writeReview(manifestId, { pull_request: item.number, kind: write.kind, ...(write.body === undefined ? {} : { body: write.body }) });
      return answer.ok ? { ok: true, page: answer.value } : { ok: false, said: refusalWords(answer.outcome) };
    },
    pullRequestPatches: async (item) => {
      const manifestId = manifestOfPr(item)?.manifest?.id;
      if (manifestId === undefined) return { ok: false, said: refusalWords({ ok: false, why: "no_manifest" } as never) };
      const answer = await api.readReviewPatches(manifestId, item.number);
      return answer.ok ? { ok: true, files: answer.value.files } : { ok: false, said: refusalWords(answer.outcome) };
    },
    actOnPullRequest: async (item, act) => {
      const manifestId = manifestOfPr(item)?.manifest?.id;
      if (manifestId === undefined) return false;
      const answer = await api.pressRepositoryPullRequest(manifestId, item.number, act).catch(() => undefined);
      return answer?.ok === true;
    },
    issue: async (item): Promise<IssueRead | undefined> => {
      const repo = repositoryOf(repositories(), { ...(item.root === undefined ? {} : { root: item.root }), ...(item.manifestId === undefined ? {} : { manifestId: item.manifestId }), ...(item.address === undefined ? {} : { address: item.address }) });
      const manifestId = repo?.manifest?.id;
      if (manifestId === undefined) return undefined;
      const answer = await api.getIssue(manifestId, issueLabel(item.number)).catch(() => undefined);
      if (answer?.ok !== true) return undefined;
      const one = answer.value;
      return {
        number: item.number,
        title: one.title,
        repository: repo?.manifest?.repository ?? "",
        address: one.url,
        state: one.state === "closed" ? "closed" : "open",
        author: one.author,
        openedAt: one.opened_at,
        labels: one.labels,
        assignees: one.assignees,
        body: one.body,
        comments: one.comments,
        linked: one.linked_pull_requests.map((pr) => ({ number: pr.number, title: pr.title, address: pr.url, state: pr.state })),
      };
    },
    bay,
    check,
    leaveCheck: () => void api.followLandCheck(null),
  };
}

/** What `pick` finds in the window's state, now or within a few seconds of it moving. */
async function until<T>(now: () => FleetPanelState | undefined, pick: (state: FleetPanelState) => T | undefined, ms = 3000): Promise<T | undefined> {
  const end = Date.now() + ms;
  for (;;) {
    const state = now();
    const found = state === undefined ? undefined : pick(state);
    if (found !== undefined || Date.now() >= end) return found;
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
}
