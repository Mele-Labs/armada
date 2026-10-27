// The review surface's state, held outside React so what was open, read or drafted stays as it was when the Reviews
// tab is left and come back to. **Built per window**, as Sessions are, and handed down by `ReviewsFrom`.
//
// Two halves with a seam between them. **The forge's** (this file): the four lists read from the forge when the tab opens
// and on Refresh and never polled, a pull request's page read when it is opened, its patches read when a change is
// shown, his pending review kept here until Submit, and resolve. **The agent's** (`store-agent.ts`): the explanations,
// Ask and the tools, which are runs Fleet keeps. Natural Search (`store-search.ts`) is a run across every repository, and the
// Walk (`store-walk.ts`) a run that needs the place a change runs. All three meet a run through `review_run.changed`.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";

import type { PullRequestPage, PullRequestState, PullReviewComment, PullReviewList, PullReviewLists, PullReviewRow } from "@armada/protocol";
import { forgetKept, readKept, writeKept } from "@armada/components";
import { SURFACE } from "@armada/shell";

import type { PullLink } from "../../../shared/pull-link";
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
  every: { state: "unread" },
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
  /**
   * Opens the pull request a link names, on whichever list it is on or on none, and reads its page. Answers its key, or the words for
   * why it was not opened: the forge would not give it, or it is no longer open.
   */
  openLink: (manifestId: string, link: PullLink) => Promise<{ ok: true; key: string } | { ok: false; said: string }>;
  /** Puts the Natural Search box up in place of the list, or takes it down. */
  setSearching: (on: boolean) => void;
  setView: (key: string, view: View) => void;
  toggleOpen: (key: string, change: string) => void;
  /** Reads one file's patch from the forge, the first time its change is shown. */
  readFile: (key: string, path: string) => void;
  /** Reads every file's patch in one call, the first time Files is shown. A refusal is asked again on the next press. */
  readPatches: (key: string) => void;
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

/** What a row says of a pull request met by link: the forge's standing, the author and head left for the page to give. */
const rowOfStanding = (state: PullRequestState, repository: string): PullReviewRow => ({
  manifest_id: state.manifest_id,
  repository,
  number: state.number,
  title: state.title,
  branch: state.branch,
  address: state.address,
  draft: state.state === "draft",
  checks: state.checks.state === "passed" ? "passed" : state.checks.state === "failed" ? "failed" : "running",
  review: "none",
  head: "",
  author: "",
  updated_at: new Date().toISOString(),
});

const keyOf = (row: PullReviewRow): string => `${row.repository}#${row.number}`;

export const KEPT_LISTS = "pr-review:lists";
/** The pull request open in the reader, so a reload opens it again rather than leaving the reader empty. */
const KEPT_OPEN = "pr-review:open";
/** The rows of pull requests opened by link, the latest first, so a reload lists them again: no list from the forge holds them. */
const KEPT_LINKED = "pr-review:linked";
const LINKED_KEPT = 10;
/** His pending comments, by pull request, until Submit is taken. */
const KEPT_DRAFTS = "pr-review:drafts";

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
    linked: [],
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
      change(key, { page: { state: "read" }, threads: page.threads ?? [], review: page.review });
      dropStale(key, head);
    });
  };

  /** A commit pushed since the patches were read makes them stale: they are read again where their change is shown. */
  const dropStale = (key: string, head: string) => {
    const { patchesHead: on, every } = work(key);
    const patchesStale = on !== undefined && on !== head;
    const everyStale = every.state === "read" && every.head !== head;
    if (!patchesStale && !everyStale) return;
    change(key, { ...(patchesStale ? { patches: {}, patchesHead: undefined } : {}), ...(everyStale ? { every: { state: "unread" as const } } : {}) });
    if (patchesStale) acts.patched(key);
  };

  /** `fresh` is a reading Fleet just answered; the one kept from the last launch is only drawn, and read again by `fetch`. */
  const arrive = (lists: PullReviewLists, fresh = true) => {
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
    if (!fresh) return;
    // Kept so the next launch draws rows at once instead of an empty panel until Fleet answers.
    writeKept(KEPT_LISTS, lists);
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

  const last = readKept<PullReviewLists>(KEPT_LISTS);
  if (last !== undefined) arrive(last, false);

  const open = (key: string | undefined) => {
    set({ open: key });
    if (key === undefined) return forgetKept(KEPT_OPEN);
    writeKept(KEPT_OPEN, key);
    readPage(key);
    acts.readRuns(key);
    const pr = prOf(key);
    if (pr !== undefined) acts.readTools(pr.manifestId);
  };

  /** A pull request opened by link joins the kept rows, at the top. */
  const keepLinked = (row: PullReviewRow) => {
    const rows = [row, ...(readKept<PullReviewRow[]>(KEPT_LINKED) ?? []).filter((one) => keyOf(one) !== keyOf(row))].slice(0, LINKED_KEPT);
    writeKept(KEPT_LINKED, rows);
    set({ linked: rows.map(keyOf) });
  };

  /** His pending comments on a pull request, changed here and kept, so a reload does not lose what he has not submitted. */
  const setDrafts = (key: string, drafts: readonly Draft[]) => {
    change(key, { drafts });
    const { [key]: _, ...others } = readKept<Record<string, readonly Draft[]>>(KEPT_DRAFTS) ?? {};
    writeKept(KEPT_DRAFTS, drafts.length === 0 ? others : { ...others, [key]: drafts });
  };

  // What he had open, what he opened by link and what he has not submitted come back after a reload as he left them.
  const linkedRows = readKept<PullReviewRow[]>(KEPT_LINKED) ?? [];
  linkedRows.forEach(adopt);
  set({ linked: linkedRows.map(keyOf) });
  for (const [key, drafts] of Object.entries(readKept<Record<string, readonly Draft[]>>(KEPT_DRAFTS) ?? {})) {
    change(key, { drafts });
    // A comment added from here on is never given the id of one kept.
    for (const one of drafts) ids = Math.max(ids, Number(one.id.replace(/^\D+/, "")) || 0);
  }
  const wasOpen = readKept<string>(KEPT_OPEN);
  if (wasOpen !== undefined && prOf(wasOpen) !== undefined) open(wasOpen);

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
    openLink: async (manifestId, link) => {
      const repository = `${link.owner}/${link.name}`;
      // The forge spells a repository as it was made; a link may not.
      const held = snapshot.prs.find((one) => one.key.toLowerCase() === `${repository}#${link.number}`.toLowerCase())?.key ?? `${repository}#${link.number}`;
      if (prOf(held) === undefined) {
        const answer = await source.pull(manifestId, link.number);
        if (!answer.ok) return { ok: false, said: answer.said };
        const standing = answer.value.state;
        if (standing === "merged" || standing === "closed") return { ok: false, said: `#${link.number} is ${standing}, and only an open pull request is reviewed here.` };
        const row = rowOfStanding(answer.value, repository);
        adopt(row);
        keepLinked(row);
      }
      open(held);
      return { ok: true, key: held };
    },
    open,
    setSearching: (searching) => set({ searching }),
    setView: (key, view) => change(key, { view }),
    toggleOpen: (key, id) => {
      const opened = work(key).opened;
      change(key, { opened: opened.includes(id) ? opened.filter((one) => one !== id) : [...opened, id] });
    },
    readFile: (key, path) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      // The lists may have moved the head before the page says so: a patch read on the new one must not keep the old ones.
      dropStale(key, pr.head);
      if (work(key).patches[path] !== undefined) return;
      const on = pr.head;
      const keep = (read: PatchRead) => change(key, { patches: { ...work(key).patches, [path]: read }, patchesHead: on });
      keep({ state: "reading" });
      void source.file(pr.manifestId, pr.number, path).then((answer) => {
        if (!answer.ok) return keep({ state: "said", said: answer.said });
        const patch = answer.value.patch;
        keep(patch === undefined ? { state: "said", said: answer.value.said ?? "No lines changed in this file." } : { state: "read", lines: linesOfPatch(path, patch) });
        acts.patched(key);
      });
    },
    readPatches: (key) => {
      const pr = prOf(key);
      if (pr === undefined) return;
      dropStale(key, pr.head);
      const held = work(key).every;
      if (held.state === "reading" || held.state === "read") return;
      const on = pr.head;
      change(key, { every: { state: "reading" } });
      void source.patches(pr.manifestId, pr.number).then((answer) => {
        change(key, { every: answer.ok ? { state: "read", head: on, files: answer.value.files } : { state: "said", said: answer.said } });
      });
    },
    addDraft: (key, draft) => setDrafts(key, [...work(key).drafts, { id: nextId("d"), ...draft }]),
    removeDraft: (key, id) => setDrafts(key, work(key).drafts.filter((one) => one.id !== id)),
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
      setDrafts(key, []);
      change(key, {
        submitting: false,
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

const NEVER: ReviewSnapshot = { prs: [], fetchedAt: undefined, fetching: false, tab: "requested", me: "", failed: undefined, finds: [], open: undefined, linked: [], searching: false, askFirst: true, spending: undefined, tools: {}, leftOut: {}, work: {}, sheet: undefined, places: { state: "unread", list: [], said: undefined } };
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
