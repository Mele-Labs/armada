// The review surface's state, held outside React so the Cockpit's review mode and the Reviews tab are one
// thing: what was open, viewed or drafted is the same whichever door it was reached by, and leaving the mode
// puts the Cockpit back without touching either. **Built per window from a seed**, as Sessions are, and handed
// down by `ReviewsFrom`. Nothing is served yet, so every act is recorded here and goes no further.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { PullRequestPage } from "@armada/protocol";
import { SURFACE } from "@armada/shell";

import type { Anchor, Finding, PrWork, ReviewPr, ReviewSeed, ReviewSnapshot, ReviewTab, Verdict } from "./model";

/** How long an agent review runs in the mock before it has read the pull request. */
export const AGENT_MS = 2200;

const EMPTY: PrWork = { viewed: [], drafts: [], threads: [], findings: [], run: { state: "idle" }, submissions: [], review: "none" };

export type ReviewStore = {
  get: () => ReviewSnapshot;
  subscribe: (listener: () => void) => () => void;
  /** The list, fetched when the review view opens and again on Refresh. Nothing polls. */
  fetch: () => void;
  setTab: (tab: ReviewTab) => void;
  open: (key: string | undefined) => void;
  setMode: (on: boolean) => void;
  toggleViewed: (key: string, path: string) => void;
  addDraft: (key: string, at: Anchor, body: string, thread?: string) => void;
  removeDraft: (key: string, id: string) => void;
  resolve: (key: string, thread: string, resolved: boolean) => void;
  /** An agent review, asked for. It runs for a moment and then its findings stand. */
  askAgent: (key: string) => void;
  dismiss: (key: string, finding: string) => void;
  sent: (key: string, findings: readonly string[]) => void;
  submit: (key: string, verdict: Verdict, body: string) => void;
};

let ids = 0;
const nextId = (prefix: string) => `${prefix}${(ids += 1)}`;

/** The store for a window, over what the forge would serve. `now` is the clock the mock's times are read by. */
export function createReviewStore(seed: ReviewSeed): ReviewStore {
  const listeners = new Set<() => void>();
  const timers = new Set<number>();
  const initialWork = (pr: ReviewPr): PrWork => ({ ...EMPTY, threads: pr.threads, review: pr.page.review });
  let snapshot: ReviewSnapshot = { prs: [], fetchedAt: undefined, fetching: false, tab: "requested", open: undefined, mode: false, work: {} };
  const set = (next: Partial<ReviewSnapshot>) => {
    snapshot = { ...snapshot, ...next };
    listeners.forEach((one) => one());
  };
  const work = (key: string): PrWork => snapshot.work[key] ?? EMPTY;
  const change = (key: string, by: Partial<PrWork>) => set({ work: { ...snapshot.work, [key]: { ...work(key), ...by } } });
  const prOf = (key: string) => snapshot.prs.find((one) => one.key === key);

  const arrive = () => {
    const kept = snapshot.work;
    set({
      prs: seed.prs,
      fetchedAt: Date.now(),
      fetching: false,
      work: Object.fromEntries(seed.prs.map((one) => [one.key, kept[one.key] ?? initialWork(one)])),
    });
  };

  return {
    get: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    fetch: () => {
      if (snapshot.fetching) return;
      set({ fetching: true });
      timers.add(window.setTimeout(arrive, snapshot.fetchedAt === undefined ? 0 : 600));
    },
    setTab: (tab) => set({ tab }),
    open: (open) => set({ open }),
    setMode: (mode) => set({ mode }),
    toggleViewed: (key, path) => {
      const viewed = work(key).viewed;
      change(key, { viewed: viewed.includes(path) ? viewed.filter((one) => one !== path) : [...viewed, path] });
    },
    addDraft: (key, at, body, thread) => change(key, { drafts: [...work(key).drafts, { id: nextId("d"), ...at, body, ...(thread === undefined ? {} : { thread }) }] }),
    removeDraft: (key, id) => change(key, { drafts: work(key).drafts.filter((one) => one.id !== id) }),
    resolve: (key, thread, resolved) => change(key, { threads: work(key).threads.map((one) => (one.id === thread ? { ...one, resolved } : one)) }),
    askAgent: (key) => {
      if (work(key).run.state === "running") return;
      change(key, { run: { state: "running", since: Date.now() }, findings: [] });
      timers.add(
        window.setTimeout(() => {
          const found: Finding[] = (seed.agent[key] ?? []).map((one) => ({ id: nextId("f"), ...one, dismissed: false, sent: false }));
          change(key, { run: { state: "done", at: Date.now() }, findings: found });
        }, AGENT_MS),
      );
    },
    dismiss: (key, finding) => change(key, { findings: work(key).findings.map((one) => (one.id === finding ? { ...one, dismissed: true } : one)) }),
    sent: (key, findings) => change(key, { findings: work(key).findings.map((one) => (findings.includes(one.id) ? { ...one, sent: true } : one)) }),
    submit: (key, verdict, body) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      const now = new Date().toISOString();
      const current = work(key);
      // Each draft is posted: a reply joins its thread, a new comment opens one.
      let threads = [...current.threads];
      for (const draft of current.drafts) {
        const comment = { id: nextId("c"), author: seed.me, at: now, body: draft.body };
        if (draft.thread !== undefined && threads.some((one) => one.id === draft.thread)) {
          threads = threads.map((one) => (one.id === draft.thread ? { ...one, comments: [...one.comments, comment] } : one));
        } else {
          threads.push({ id: nextId("t"), path: draft.path, line: draft.line, resolved: false, comments: [comment] });
        }
      }
      const review: PullRequestPage["review"] = verdict === "approve" ? "approved" : verdict === "request_changes" ? "changes_requested" : current.review;
      change(key, {
        threads,
        drafts: [],
        review,
        submissions: [...current.submissions, { verdict, body: body.trim(), comments: current.drafts.length, at: now }],
      });
    },
  };
}

// ---- The window's hand-down -----------------------------------------------------------------------------

const Held = createContext<ReviewStore | undefined>(undefined);

/** Hands the window the store a scenario built. Nothing handed, and no review surface is offered. */
export function ReviewsFrom({ store, children }: { store: ReviewStore | undefined; children: ReactNode }) {
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

export const useReviewStore = (): ReviewStore | undefined => useContext(Held);

const NEVER: ReviewSnapshot = { prs: [], fetchedAt: undefined, fetching: false, tab: "requested", open: undefined, mode: false, work: {} };
const NO_SUBSCRIPTION = () => () => undefined;
const NO_SNAPSHOT = () => NEVER;

/** The review state as it stands, live. Empty where nothing serves reviews. */
export function useReviews(): ReviewSnapshot {
  const store = useContext(Held);
  return useSyncExternalStore(store?.subscribe ?? NO_SUBSCRIPTION, store?.get ?? NO_SNAPSHOT);
}

/** The surfaces the rail and the palette leave off: Reviews, until something serves them. */
export function reviewsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.reviews];
}

export { EMPTY as NO_WORK };
