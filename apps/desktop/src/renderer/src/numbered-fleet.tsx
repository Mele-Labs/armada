// What `#` reads on a real Fleet. A repository's open issues are `list_issues`, asked the first time
// a box asks for them and again once they are stale; its open pull requests are the merge line's
// `hub.pull_requests`, which the window already holds. A number that is neither is looked up by
// number, `get_pull_request` first and `get_issue` after it. The mapping to a tag is `numbered.tsx`'s.
//
// **A Manifest id is joined to a merge line through the repository's root**: `holds.repositories`
// lists both, and a line is named by the root alone.

import { useEffect, useMemo } from "react";
import type { ReactNode } from "react";

import type { NumberedHit } from "@armada/components";
import type { HubPullRequest, RepositorySummary } from "@armada/protocol";

import type { BridgeApi } from "../../shared/api";
import type { BridgeState } from "../../shared/bridge";
import { withState, type StateSource } from "./state-feed";
import { issueHit, lookedUpPullRequest, NumberedFrom, pullRequestHit } from "./numbered";
import type { NumberedSource } from "./numbered";
import { jobIdOf } from "@armada/shell";

/** What a list of issues is worth before it is asked for again. `#` asks for it each time it opens. */
export const STALE_MS = 30_000;

const NONE: readonly never[] = [];

type Reads = StateSource & Pick<BridgeApi, "listIssues" | "getIssue" | "getPullRequest">;

type Issues = { at: number; loading: boolean; hits: readonly NumberedHit[] };

/** The answer `open` last gave a repository, kept while neither of its two parts moved. */
type Joined = { issues: readonly NumberedHit[]; pulls: readonly HubPullRequest[] | undefined; hits: readonly NumberedHit[] };

/** The store behind one window's `#`. Exported for a test that holds it without a window. */
export class FleetNumbered {
  private readonly api: Reads;
  private readonly now: () => number;
  private repositories: readonly RepositorySummary[] = NONE;
  private lines: BridgeState["mergeLines"] = null;
  private readonly issues = new Map<string, Issues>();
  private readonly joined = new Map<string, Joined>();
  private readonly listeners = new Set<() => void>();
  private moved = 0;

  constructor(api: Reads, now: () => number = Date.now) {
    this.api = api;
    this.now = now;
  }

  /** Read the state and follow it. Returns the way to stop. */
  attach(): () => void {
    let live = true;
    void this.api.state().then((state) => live && this.take(state));
    const off = this.api.subscribe((state) => this.take(state));
    return () => {
      live = false;
      off();
    };
  }

  /** Follow the repositories and the merge lines; tell the boxes only when a repository they asked for has other pull requests. */
  take(state: Pick<BridgeState, "holds" | "mergeLines">): void {
    const was = [...this.issues.keys()].map((id) => this.pullsOf(id));
    this.repositories = state.holds.repositories ?? NONE;
    this.lines = state.mergeLines;
    const now = [...this.issues.keys()].map((id) => this.pullsOf(id));
    if (was.some((pulls, at) => pulls !== now[at])) this.changed();
  }

  private pullsOf(manifestId: string): readonly HubPullRequest[] | undefined {
    const root = this.repositories.find((one) => jobIdOf(one) === manifestId)?.root;
    return root === undefined ? undefined : this.lines?.lines.find((one) => one.root === root)?.hub?.pull_requests;
  }

  private changed(): void {
    this.moved += 1;
    this.listeners.forEach((listener) => listener());
  }

  /** The issues of a repository: asked for the first time here, and once more when what is held has gone stale. */
  private ask(manifestId: string): Issues {
    const held = this.issues.get(manifestId) ?? { at: Number.NEGATIVE_INFINITY, loading: false, hits: NONE };
    if (held.loading || this.now() - held.at <= STALE_MS) return held;
    held.at = this.now();
    held.loading = true;
    this.issues.set(manifestId, held);
    void this.api
      .listIssues(manifestId, false)
      .then((read) => {
        held.loading = false;
        if (!read.ok) return;
        held.hits = read.value.issues.map(issueHit);
        this.changed();
      })
      .catch(() => {
        held.loading = false;
      });
    return held;
  }

  private readonly open = (manifestId: string | undefined): readonly NumberedHit[] => {
    if (manifestId === undefined) return NONE;
    const { hits: issues } = this.ask(manifestId);
    const pulls = this.pullsOf(manifestId);
    const kept = this.joined.get(manifestId);
    if (kept !== undefined && kept.issues === issues && kept.pulls === pulls) return kept.hits;
    const hits = [...issues, ...(pulls ?? NONE).map(pullRequestHit)];
    this.joined.set(manifestId, { issues, pulls, hits });
    return hits;
  };

  private readonly find = async (manifestId: string | undefined, number: number): Promise<NumberedHit | undefined> => {
    if (manifestId === undefined) return undefined;
    // A pull request first, open, merged or closed; a number the forge has no pull request for is tried as an issue.
    const pull = await this.api.getPullRequest(manifestId, number).catch(() => undefined);
    if (pull?.ok === true) return lookedUpPullRequest(pull.value);
    const issue = await this.api.getIssue(manifestId, `#${number}`).catch(() => undefined);
    if (issue?.ok !== true) return undefined;
    const hit = issueHit(issue.value);
    return issue.value.state === "closed" ? { ...hit, standing: "Closed" } : hit;
  };

  readonly source: NumberedSource = {
    open: this.open,
    find: this.find,
    subscribe: (listener) => {
      this.listeners.add(listener);
      return () => void this.listeners.delete(listener);
    },
    version: () => this.moved,
  };
}

/** `#` over a real Fleet, for the window the app mounts. The mock mounts its own. */
export function WiredNumbered({ children }: { children: ReactNode }) {
  const store = useMemo(() => new FleetNumbered(withState(window.armada)), []);
  useEffect(() => store.attach(), [store]);
  return <NumberedFrom held={store.source}>{children}</NumberedFrom>;
}
