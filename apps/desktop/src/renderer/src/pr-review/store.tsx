// The review surface's state, held outside React so the Cockpit's review mode and the Reviews tab are one
// thing: what was open, run or drafted is the same whichever door it was reached by, and leaving the mode
// puts the Cockpit back without touching either. **Built per window from a seed**, as Sessions are, and handed
// down by `ReviewsFrom`. Nothing is served yet, so every act is recorded here and goes no further.
//
// **An agent runs only when he presses**: Explain on a pull request nobody of his wrote, and each tool. Both
// run for this pull request alone, and a result stands until he asks again.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { PullRequestPage } from "@armada/protocol";
import { SURFACE } from "@armada/shell";

import type { Anchor, Asked, Find, Finding, PrWork, ReviewPr, ReviewSeed, ReviewSnapshot, ReviewTab, ToolDef, Verdict, View } from "./model";
import { bySession, idOfName } from "./tools";

/** How long an agent runs in the mock before it has read the pull request. */
export const AGENT_MS = 2200;

/** A pull request with the commit pushed to it since: its head moves, and the page lists it. */
const withPush = (pr: ReviewPr, push: ReviewSeed["pushes"][string] | undefined): ReviewPr =>
  push === undefined
    ? pr
    : { ...pr, head: push.sha, page: { ...pr.page, commits: [...pr.page.commits, { sha: push.sha, headline: push.headline, author: push.author, at: new Date().toISOString() }] } };

const EMPTY: PrWork = { explains: {}, kind: undefined, asks: [], view: "explanation", runs: {}, opened: [], drafts: [], threads: [], findings: [], submissions: [], review: "none" };

export type ToolIn = { kind: ToolDef["kind"]; name: string; checks: string; scope: "kit" | "repository"; manifestId: string };

export type ReviewStore = {
  get: () => ReviewSnapshot;
  subscribe: (listener: () => void) => () => void;
  /** The list, fetched when the review view opens and again on Refresh. Nothing polls. */
  fetch: () => void;
  /** Chooses the list: `requested`, `mine`, or a saved find's id. */
  setTab: (tab: ReviewTab) => void;
  /** Asks a Drone to look for what he described. Runs on send only, since it costs, and the find is saved as a list. */
  find: (words: string) => void;
  rerunFind: (id: string) => void;
  removeFind: (id: string) => void;
  open: (key: string | undefined) => void;
  setMode: (on: boolean) => void;
  setView: (key: string, view: View) => void;
  /** Chooses an explanation. An agent reads the pull request the first time only, unless a Session of his already knows. A kept one shows at once. */
  explain: (key: string, def: ToolDef) => void;
  /** Explains again, at the commit the pull request is at now. */
  explainAgain: (key: string, def: ToolDef) => void;
  /** Shows a choice under the description: an explanation's id, `ask`, or one of his questions. Runs nothing. */
  show: (key: string, kind: string) => void;
  /** Asks a question about the pull request, in his words. Runs on send only, and is kept under them. */
  ask: (key: string, words: string) => void;
  askAgain: (key: string, id: string) => void;
  removeAsk: (key: string, id: string) => void;
  /** Runs a tool for this pull request, or shows what it found last time. Comments runs nothing. */
  runTool: (key: string, tool: ToolDef) => void;
  /** Asks a tool again, for this pull request. */
  rerun: (key: string, tool: ToolDef) => void;
  toggleOpen: (key: string, change: string) => void;
  addDraft: (key: string, at: Anchor, body: string, thread?: string) => void;
  removeDraft: (key: string, id: string) => void;
  resolve: (key: string, thread: string, resolved: boolean) => void;
  dismiss: (key: string, finding: string) => void;
  sent: (key: string, findings: readonly string[]) => void;
  /** `told` is the Session a Request changes was sent to as well, by title. */
  submit: (key: string, verdict: Verdict, body: string, told?: string) => void;
  /** Records a tool he wrote, in his Kit or in the repository. Nothing is written to a file in the mock. */
  addTool: (one: ToolIn) => void;
};

let ids = 0;
const nextId = (prefix: string) => `${prefix}${(ids += 1)}`;

/** The store for a window, over what the forge would serve. */
export function createReviewStore(seed: ReviewSeed): ReviewStore {
  const listeners = new Set<() => void>();
  const timers = new Set<number>();
  const initialWork = (pr: ReviewPr): PrWork => ({
    ...EMPTY,
    explains: pr.explainedBy === "session" ? { "how-it-works": { state: "done", at: Date.now(), head: pr.head }, why: { state: "done", at: Date.now(), head: pr.head } } : {},
    kind: pr.explainedBy === "session" ? "how-it-works" : undefined,
    asks: pr.asked.map((words) => ({ id: nextId("ask"), words, run: { state: "done", at: Date.now(), head: pr.head } })),
    threads: pr.threads,
    review: pr.page.review,
  });
  let snapshot: ReviewSnapshot = {
    prs: [],
    fetchedAt: undefined,
    fetching: false,
    tab: "requested",
    finds: [],
    open: undefined,
    mode: false,
    armada: seed.tools.armada,
    kit: seed.tools.kit,
    repository: seed.tools.repository,
    work: {},
  };
  const set = (next: Partial<ReviewSnapshot>) => {
    snapshot = { ...snapshot, ...next };
    listeners.forEach((one) => one());
  };
  const work = (key: string): PrWork => snapshot.work[key] ?? EMPTY;
  const change = (key: string, by: Partial<PrWork>) => set({ work: { ...snapshot.work, [key]: { ...work(key), ...by } } });
  const prOf = (key: string) => snapshot.prs.find((one) => one.key === key);

  const arrive = () => {
    const kept = snapshot.work;
    // A commit pushed since he first read the list arrives with the second read.
    const prs = snapshot.fetchedAt === undefined ? seed.prs : seed.prs.map((one) => withPush(one, seed.pushes[one.key]));
    set({
      prs,
      fetchedAt: Date.now(),
      fetching: false,
      work: Object.fromEntries(prs.map((one) => [one.key, kept[one.key] ?? initialWork(one)])),
    });
  };

  const setFind = (id: string, by: Partial<Find>) => set({ finds: snapshot.finds.map((one) => (one.id === id ? { ...one, ...by } : one)) });
  const look = (id: string, words: string) => {
    setFind(id, { run: { state: "running", since: Date.now() }, hits: [] });
    timers.add(
      window.setTimeout(() => {
        const hits = seed.finds.find((one) => new RegExp(one.when, "i").test(words))?.hits ?? [];
        setFind(id, { run: { state: "done", at: Date.now(), head: "" }, hits });
      }, AGENT_MS + 600),
    );
  };

  const start = (key: string, tool: ToolDef) => {
    const head = prOf(key)?.head ?? "";
    change(key, {
      view: tool.id,
      runs: { ...work(key).runs, [tool.id]: { state: "running", since: Date.now() } },
      findings: work(key).findings.filter((one) => one.tool !== tool.id),
    });
    timers.add(
      window.setTimeout(() => {
        const found: Finding[] = (seed.results[key]?.[tool.id] ?? []).map((one) => ({ id: nextId("f"), tool: tool.id, ...one, dismissed: false, sent: false }));
        change(key, { runs: { ...work(key).runs, [tool.id]: { state: "done", at: Date.now(), head } }, findings: [...work(key).findings, ...found] });
      }, AGENT_MS),
    );
  };

  const read = (key: string, def: ToolDef, head: string) => {
    change(key, { kind: def.id, explains: { ...work(key).explains, [def.id]: { state: "running", since: Date.now() } } });
    timers.add(window.setTimeout(() => change(key, { explains: { ...work(key).explains, [def.id]: { state: "done", at: Date.now(), head } } }), AGENT_MS));
  };

  const setAsk = (key: string, id: string, run: Asked["run"]) => change(key, { asks: work(key).asks.map((one) => (one.id === id ? { ...one, run } : one)) });
  const answer = (key: string, id: string, head: string) => {
    setAsk(key, id, { state: "running", since: Date.now() });
    timers.add(window.setTimeout(() => setAsk(key, id, { state: "done", at: Date.now(), head }), AGENT_MS));
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
    find: (words) => {
      const said = words.trim().replace(/\s+/g, " ");
      if (said === "") return;
      const again = snapshot.finds.find((one) => one.words.toLowerCase() === said.toLowerCase());
      const id = again?.id ?? nextId("find");
      if (again === undefined) set({ finds: [...snapshot.finds, { id, words: said, run: { state: "idle" }, hits: [] }] });
      set({ tab: id });
      if (again?.run.state !== "running") look(id, said);
    },
    rerunFind: (id) => {
      const one = snapshot.finds.find((one) => one.id === id);
      if (one !== undefined && one.run.state !== "running") look(id, one.words);
    },
    removeFind: (id) => set({ finds: snapshot.finds.filter((one) => one.id !== id), tab: snapshot.tab === id ? "requested" : snapshot.tab }),
    open: (open) => set({ open }),
    setMode: (mode) => set({ mode }),
    setView: (key, view) => change(key, { view }),
    explain: (key, def) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      const kept = work(key).explains[def.id];
      change(key, { kind: def.id });
      if (bySession(pr, def) || (kept !== undefined && kept.state !== "idle")) return;
      read(key, def, pr.head);
    },
    explainAgain: (key, def) => {
      const pr = prOf(key);
      if (pr === undefined || bySession(pr, def) || work(key).explains[def.id]?.state === "running") return;
      read(key, def, pr.head);
    },
    show: (key, kind) => change(key, { kind }),
    ask: (key, words) => {
      const pr = prOf(key);
      const said = words.trim().replace(/\s+/g, " ");
      if (pr === undefined || said === "") return;
      const again = work(key).asks.find((one) => one.words.toLowerCase() === said.toLowerCase());
      const id = again?.id ?? nextId("ask");
      if (again === undefined) change(key, { asks: [...work(key).asks, { id, words: said, run: { state: "idle" } }] });
      change(key, { kind: id });
      if (again?.run.state !== "running") answer(key, id, pr.head);
    },
    askAgain: (key, id) => {
      const pr = prOf(key);
      if (pr !== undefined && work(key).asks.find((one) => one.id === id)?.run.state !== "running") answer(key, id, pr.head);
    },
    removeAsk: (key, id) => change(key, { asks: work(key).asks.filter((one) => one.id !== id), kind: work(key).kind === id ? "ask" : work(key).kind }),
    runTool: (key, tool) => {
      if (!tool.agent) return change(key, { view: tool.id });
      const run = work(key).runs[tool.id];
      if (run === undefined || run.state === "idle") return start(key, tool);
      change(key, { view: tool.id });
    },
    rerun: (key, tool) => {
      if (work(key).runs[tool.id]?.state === "running") return;
      start(key, tool);
    },
    toggleOpen: (key, id) => {
      const opened = work(key).opened;
      change(key, { opened: opened.includes(id) ? opened.filter((one) => one !== id) : [...opened, id] });
    },
    addDraft: (key, at, body, thread) => change(key, { drafts: [...work(key).drafts, { id: nextId("d"), ...at, body, ...(thread === undefined ? {} : { thread }) }] }),
    removeDraft: (key, id) => change(key, { drafts: work(key).drafts.filter((one) => one.id !== id) }),
    resolve: (key, thread, resolved) => change(key, { threads: work(key).threads.map((one) => (one.id === thread ? { ...one, resolved } : one)) }),
    dismiss: (key, finding) => change(key, { findings: work(key).findings.map((one) => (one.id === finding ? { ...one, dismissed: true } : one)) }),
    sent: (key, findings) => change(key, { findings: work(key).findings.map((one) => (findings.includes(one.id) ? { ...one, sent: true } : one)) }),
    submit: (key, verdict, body, told) => {
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
          threads.push({ id: nextId("t"), change: draft.change, hunk: draft.hunk, ...(draft.line === undefined ? {} : { line: draft.line }), resolved: false, comments: [comment] });
        }
      }
      const review: PullRequestPage["review"] = verdict === "approve" ? "approved" : verdict === "request_changes" ? "changes_requested" : current.review;
      change(key, {
        threads,
        drafts: [],
        review,
        submissions: [...current.submissions, { verdict, body: body.trim(), comments: current.drafts.length, at: now, ...(told === undefined ? {} : { told }) }],
      });
    },
    addTool: ({ kind, name, checks, scope, manifestId }) => {
      const def: ToolDef = { id: idOfName(name), kind, name: name.trim(), source: scope, mark: "custom", checks: checks.trim(), agent: true };
      if (scope === "kit") set({ kit: [...snapshot.kit.filter((one) => one.id !== def.id), def] });
      else set({ repository: { ...snapshot.repository, [manifestId]: [...(snapshot.repository[manifestId] ?? []).filter((one) => one.id !== def.id), def] } });
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

const NEVER: ReviewSnapshot = { prs: [], fetchedAt: undefined, fetching: false, tab: "requested", finds: [], open: undefined, mode: false, armada: [], kit: [], repository: {}, work: {} };
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
