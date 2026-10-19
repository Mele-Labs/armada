// The review surface's state, held outside React so what was open, read or drafted stays as it was when the Reviews
// tab is left and come back to. **Built per window**, as Sessions are, and handed down by `ReviewsFrom`.
//
// Two halves with a seam between them. **The forge's** (this file): the four lists read from the forge when the tab opens
// and on Refresh and never polled, a pull request's page read when it is opened, its patches read when a change is
// shown, his pending review kept here until Submit, and resolve. **The agent's** (`store-agent.ts`): the quick actions,
// Ask and the tools, which are runs Fleet keeps. Natural Search (`store-search.ts`) is a run across every repository, and the
// Walk (`store-walk.ts`) a run that needs the place a change runs. All three meet a run through `review_run.changed`.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { PullRequestPage, PullReviewComment, PullReviewList, PullReviewLists, PullReviewRow } from "@armada/protocol";
import { SURFACE } from "@armada/shell";

import type { Draft, PatchRead, PrList, PrWork, ReviewPr, ReviewSnapshot, ReviewTab, ToolDef, Verdict, View, WalkWork } from "./model";
import { linesOfPatch } from "./patch";
import type { ReviewSource } from "./source";
import { createAgentActs, type AgentActs } from "./store-agent";
import { createSearchActs, type SearchActs } from "./store-search";
import { createWalkActs, type WalkActs } from "./store-walk";

const EMPTY_WALK: WalkWork = { find: { state: "idle" }, asking: false, held: undefined, reached: undefined, opening: false, said: undefined, notes: [] };

const EMPTY: PrWork = {
  page: { state: "unread" },
  patches: {},
  refused: undefined,
  submitting: false,
  explains: {},
  accounts: {},
  wire: {},
  kind: undefined,
  asks: [],
  view: "explanation",
  runs: {},
  opened: [],
  drafts: [],
  threads: [],
  findings: [],
  submissions: [],
  review: "none",
  walk: EMPTY_WALK,
};

export type ToolIn = { kind: ToolDef["kind"]; name: string; checks: string; scope: "kit" | "repository"; manifestId: string };

export type ReviewStore = AgentActs &
  SearchActs &
  WalkActs & {
  get: () => ReviewSnapshot;
  subscribe: (listener: () => void) => () => void;
  /** The lists, read from the forge when the review view opens and again on Refresh. Nothing polls. */
  fetch: () => void;
  /** Chooses the list: `requested`, `mine`, or a saved find's id. */
  setTab: (tab: ReviewTab) => void;
  /** Opens a pull request in the reader, and reads its page from the forge. */
  open: (key: string | undefined) => void;
  /** Takes a pull request that is on none of the lists, as a search does, so the review tools can run on it. Answers its key. */
  adopt: (row: PullReviewRow) => string;
  /** Puts the Natural Search box up in place of the list, or takes it down. */
  setSearching: (on: boolean) => void;
  setView: (key: string, view: View) => void;
  toggleOpen: (key: string, change: string) => void;
  /** Reads one file's patch from the forge, the first time its change is shown. */
  readFile: (key: string, path: string) => void;
  addDraft: (key: string, draft: Omit<Draft, "id">) => void;
  removeDraft: (key: string, id: string) => void;
  /** Resolves a thread, or opens it again, at the forge. Answers whether the forge took it; a refusal is in `refused`. */
  resolve: (key: string, thread: string, resolved: boolean) => Promise<boolean>;
  /**
   * Posts his pending review: the verdict, its summary and every draft, as one review at the commit he read. Answers whether the
   * forge took it. **Nothing is cleared on a refusal**: his drafts stay, and the forge's words are in `refused`.
   * `told` is the Session a Request changes was sent to as well, by title.
   */
  submit: (key: string, verdict: Verdict, body: string, told?: string) => Promise<boolean>;
};

let ids = 0;
const nextId = (prefix: string) => `${prefix}${(ids += 1)}`;

/** A pull request until its page is read: only what the row says. */
const stubPage = (row: PullReviewRow): PullRequestPage => ({
  manifest_id: row.manifest_id,
  number: row.number,
  base: "",
  author: row.author,
  additions: 0,
  deletions: 0,
  mergeable: "unknown",
  review: row.review,
  approvals: 0,
  conversation: [],
  checks: [],
  files: [],
  commits: [],
});

const keyOf = (row: PullReviewRow): string => `${row.repository}#${row.number}`;

const LISTS: readonly PrList[] = ["requested", "team", "assigned", "mine"];

/** The store for a window, over what Fleet serves. */
export function createReviewStore(source: ReviewSource): ReviewStore {
  const listeners = new Set<() => void>();
  /** Pull requests a search turned up that no list holds: they stay while the lists are read again. */
  const adopted = new Set<string>();
  let snapshot: ReviewSnapshot = {
    prs: [],
    fetchedAt: undefined,
    fetching: false,
    tab: "requested",
    me: "",
    failed: undefined,
    finds: [],
    open: undefined,
    searching: false,
    askFirst: true,
    spending: undefined,
    tools: {},
    leftOut: {},
    work: {},
    sheet: undefined,
    places: { state: "unread", list: [], said: undefined },
  };
  const set = (next: Partial<ReviewSnapshot>) => {
    snapshot = { ...snapshot, ...next };
    listeners.forEach((one) => one());
  };
  const work = (key: string): PrWork => snapshot.work[key] ?? EMPTY;
  const change = (key: string, by: Partial<PrWork>) => set({ work: { ...snapshot.work, [key]: { ...work(key), ...by } } });
  const prOf = (key: string) => snapshot.prs.find((one) => one.key === key);
  const mend = (key: string, by: Partial<ReviewPr>) => set({ prs: snapshot.prs.map((one) => (one.key === key ? { ...one, ...by } : one)) });

  const prOfRow = (row: PullReviewRow, lists: readonly PrList[], was: ReviewPr | undefined): ReviewPr => {
    const key = keyOf(row);
    return {
      key,
      repository: row.repository,
      manifestId: row.manifest_id,
      number: row.number,
      title: row.title,
      branch: row.branch,
      address: row.address,
      draft: row.draft,
      checks: row.checks,
      head: row.head,
      updatedAt: row.updated_at,
      lists,
      ...(row.team === undefined ? {} : { team: row.team }),
      ...(row.holder === undefined ? {} : { holder: row.holder }),
      body: was?.body ?? "",
      page: was?.page ?? stubPage(row),
      explanation: was?.explanation ?? { summary: "", themes: [] },
    };
  };

  /** Takes a pull request a search turned up, which may be on none of the four lists, so it opens like any other. */
  const adopt = (row: PullReviewRow): string => {
    const key = keyOf(row);
    if (prOf(key) !== undefined) return key;
    adopted.add(key);
    set({ prs: [...snapshot.prs, prOfRow(row, [], undefined)].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt)), work: { ...snapshot.work, [key]: { ...EMPTY, review: row.review } } });
    acts.readTools(row.manifest_id);
    return key;
  };

  const hold = { get: () => snapshot, set, work, change, mend, prOf, nextId, adopt };
  const acts = createAgentActs(source, hold);
  const search = createSearchActs(source, hold);
  const walks = createWalkActs(source, hold, acts);

  /** The page of a pull request, read from the forge. A page already held stays on screen while it is read again. */
  const readPage = (key: string) => {
    const pr = prOf(key);
    if (pr === undefined || work(key).page.state === "reading") return;
    const held = work(key).page.state === "read";
    if (!held) change(key, { page: { state: "reading" } });
    void source.page(pr.manifestId, pr.number).then((answer) => {
      const now = prOf(key);
      if (now === undefined) return;
      if (!answer.ok) return change(key, { page: held ? { state: "read" } : { state: "said", said: answer.said } });
      const page = answer.value;
      const head = page.head ?? now.head;
      mend(key, { page, body: page.body ?? "", head });
      // A commit pushed since the patches were read makes them stale: they are read again where their change is shown.
      change(key, { page: { state: "read" }, threads: page.threads ?? [], review: page.review, ...(head === now.head ? {} : { patches: {} }) });
      if (head !== now.head) acts.patched(key);
    });
  };

  const arrive = (lists: PullReviewLists) => {
    const held = snapshot.prs;
    const silent = new Set<PullReviewList>(lists.silent ?? []);
    const byKey = new Map<string, { row: PullReviewRow; on: PrList[] }>();
    for (const list of LISTS) {
      for (const row of lists[list]) {
        const one = byKey.get(keyOf(row)) ?? { row, on: [] };
        one.on.push(list);
        // The team that was asked is only on the Team list's row, whichever list the pull request was met on first.
        if (row.team !== undefined && one.row.team === undefined) one.row = { ...one.row, team: row.team };
        byKey.set(keyOf(row), one);
      }
    }
    const arrived = [...byKey.values()].map(({ row, on }) => {
      const was = held.find((one) => one.key === keyOf(row));
      // A list the forge would not answer keeps what it last held, so one failure does not blank a list.
      const kept = (was?.lists ?? []).filter((list) => silent.has(list));
      return prOfRow(row, [...new Set([...on, ...kept])], was);
    });
    // A pull request only on a silent list stays where it was, and so does one a search turned up that no list holds.
    const carried = held.filter((one) => !byKey.has(one.key) && (adopted.has(one.key) || one.lists.some((list) => silent.has(list)))).map((one) => ({ ...one, lists: one.lists.filter((list) => silent.has(list)) }));
    const prs = [...arrived, ...carried].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
    const kept = snapshot.work;
    const fetchedAt = Date.parse(lists.fetched_at);
    set({
      prs,
      me: lists.me,
      failed: undefined,
      fetchedAt: Number.isNaN(fetchedAt) ? Date.now() : fetchedAt,
      fetching: false,
      work: Object.fromEntries(
        prs.map((one) => {
          const review = byKey.get(one.key)?.row.review ?? kept[one.key]?.review ?? "none";
          return [one.key, kept[one.key] === undefined ? { ...EMPTY, review } : { ...kept[one.key]!, review }];
        }),
      ),
    });
    for (const manifestId of new Set(prs.map((one) => one.manifestId))) acts.readTools(manifestId);
    // The pull request open in the reader is read again, since a commit may have been pushed to it, and so are its runs.
    if (snapshot.open !== undefined && prOf(snapshot.open) !== undefined) {
      readPage(snapshot.open);
      acts.readRuns(snapshot.open);
    }
  };

  // A search has no pull request, so it is routed before the lookup by one.
  source.onRun((run) => (run.kind === "search" ? search.applySearch(run) : acts.apply(run)));
  source.onWalk(walks.applyWalk);
  source.onNote(walks.noted);
  source.onClosed(walks.closed);

  return {
    ...acts,
    ...search,
    ...walks,
    get: () => snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    fetch: () => {
      if (snapshot.fetching) return;
      set({ fetching: true });
      void source.lists().then((answer) => (answer.ok ? arrive(answer.value) : set({ fetching: false, failed: answer.said })));
      // The searches are read with the lists, since a search's progress is not replayed.
      search.readSearches();
    },
    setTab: (tab) => set({ tab }),
    adopt,
    open: (open) => {
      set({ open });
      if (open === undefined) return;
      readPage(open);
      acts.readRuns(open);
      const pr = prOf(open);
      if (pr !== undefined) acts.readTools(pr.manifestId);
    },
    setSearching: (searching) => set({ searching }),
    setView: (key, view) => change(key, { view }),
    toggleOpen: (key, id) => {
      const opened = work(key).opened;
      change(key, { opened: opened.includes(id) ? opened.filter((one) => one !== id) : [...opened, id] });
    },
    readFile: (key, path) => {
      const pr = prOf(key);
      if (pr === undefined || work(key).patches[path] !== undefined) return;
      const keep = (read: PatchRead) => change(key, { patches: { ...work(key).patches, [path]: read } });
      keep({ state: "reading" });
      void source.file(pr.manifestId, pr.number, path).then((answer) => {
        if (!answer.ok) return keep({ state: "said", said: answer.said });
        const patch = answer.value.patch;
        keep(patch === undefined ? { state: "said", said: answer.value.said ?? "No lines changed in this file." } : { state: "read", lines: linesOfPatch(path, patch) });
        acts.patched(key);
      });
    },
    addDraft: (key, draft) => change(key, { drafts: [...work(key).drafts, { id: nextId("d"), ...draft }] }),
    removeDraft: (key, id) => change(key, { drafts: work(key).drafts.filter((one) => one.id !== id) }),
    resolve: async (key, thread, resolved) => {
      const pr = prOf(key);
      if (pr === undefined) return false;
      const answer = await source.resolve(pr.manifestId, { thread, resolved });
      if (!answer.ok) {
        change(key, { refused: answer.said });
        return false;
      }
      change(key, { refused: undefined, threads: work(key).threads.map((one) => (one.id === thread ? { ...one, resolved: answer.value.resolved } : one)) });
      return true;
    },
    submit: async (key, verdict, body, told) => {
      const pr = prOf(key);
      const current = work(key);
      if (pr === undefined || current.submitting) return false;
      const comments: PullReviewComment[] = current.drafts.map((one) =>
        one.thread !== undefined
          ? { kind: "reply", thread: one.thread, body: one.body }
          : one.line !== undefined && one.side !== undefined
            ? { kind: "line", path: one.path, line: one.line, side: one.side, body: one.body }
            : { kind: "file", path: one.path, body: one.body },
      );
      change(key, { submitting: true, refused: undefined });
      const answer = await source.write(pr.manifestId, {
        pull_request: pr.number,
        kind: verdict,
        ...(body.trim() === "" ? {} : { body: body.trim() }),
        ...(comments.length === 0 ? {} : { comments }),
        ...(pr.head === "" ? {} : { commit: pr.head }),
      });
      if (!answer.ok) {
        change(key, { submitting: false, refused: answer.said });
        return false;
      }
      const page = answer.value;
      mend(key, { page, body: page.body ?? pr.body, head: page.head ?? pr.head });
      change(key, {
        submitting: false,
        drafts: [],
        threads: page.threads ?? [],
        review: page.review,
        submissions: [...work(key).submissions, { verdict, body: body.trim(), comments: comments.length, at: new Date().toISOString(), ...(told === undefined ? {} : { told }) }],
      });
      return true;
    },
  };
}

// ---- The window's hand-down -----------------------------------------------------------------------------

const Held = createContext<ReviewStore | undefined>(undefined);

/** Hands the window its store. Nothing handed, and no review surface is offered. */
export function ReviewsFrom({ store, children }: { store: ReviewStore | undefined; children: ReactNode }) {
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

export const useReviewStore = (): ReviewStore | undefined => useContext(Held);

const NEVER: ReviewSnapshot = { prs: [], fetchedAt: undefined, fetching: false, tab: "requested", me: "", failed: undefined, finds: [], open: undefined, searching: false, askFirst: true, spending: undefined, tools: {}, leftOut: {}, work: {}, sheet: undefined, places: { state: "unread", list: [], said: undefined } };
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
