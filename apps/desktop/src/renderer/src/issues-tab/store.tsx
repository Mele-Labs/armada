// The Issues tab's state for one window: the trackers, each query's issues as last run, the issue open in the reader, the
// box a query is described in and what the model wrote, the saved query being edited, and the spend being asked about.
// **A query runs when a list needs it and again only when it changes**: nothing polls a tracker. Writing a query is the one
// act that calls a model, so it asks first, through the box every spending act uses.

import { createContext, useContext, useEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { SURFACE } from "@armada/shell";

import type { IssuesSource, ReportedBug, SavedQuery, Ticket, TicketRead, Tracker, WrittenQuery } from "./model";

export type Run = { state: "running" } | { state: "read"; tickets: Ticket[] } | { state: "failed"; said: string; /** The program the tracker could not run: the line offers to install it. */ missing?: string; /** The program the tracker found no project for: the line offers to set it up. */ needsSetup?: string };

/** One install of a missing program, by repository and tool: under way, or refused with why. A finished one is gone. */
export type Install = { state: "installing" } | { state: "failed"; said: string };

/** The box, and once he sends it, what the model wrote for each repository. */
export type Describing = { phase: "box" } | { phase: "written"; words: string; written: Record<string, { state: "running" } | { state: "read"; query: WrittenQuery } | { state: "failed"; said: string }> };

export type IssuesSnapshot = {
  trackers: readonly Tracker[] | undefined;
  failed: string | undefined;
  runs: Readonly<Record<string, Run>>;
  installs: Readonly<Record<string, Install>>;
  reads: Readonly<Record<string, Read>>;
  open: { manifestId: string; reference: string } | undefined;
  /** The bugs agents reported, and which of them this window has opened (an unopened one is unread). */
  reports: readonly ReportedBug[];
  seen: readonly string[];
  /** The report in the reader, by id. It and `open` are never both set. */
  openReport: string | undefined;
  describing: Describing | undefined;
  /** The saved query being edited by hand, as `manifestId` and its id. */
  editing: { manifestId: string; id: string } | undefined;
  spending: string | undefined;
  askFirst: boolean;
};

type Read = { state: "read"; ticket: TicketRead } | { state: "failed"; said: string };

const key = (manifestId: string, rest: string): string => `${manifestId}\n${rest}`;

export type IssuesStore = ReturnType<typeof createIssuesStore>;

export function createIssuesStore(source: IssuesSource) {
  let snap: IssuesSnapshot = { trackers: undefined, failed: undefined, runs: {}, installs: {}, reads: {}, open: undefined, reports: [], seen: [], openReport: undefined, describing: undefined, editing: undefined, spending: undefined, askFirst: true };
  const listeners = new Set<() => void>();
  const set = (next: Partial<IssuesSnapshot>) => {
    snap = { ...snap, ...next };
    for (const one of listeners) one();
  };
  let waiting: (() => void) | undefined;
  /** The repositories whose tracker has been asked for, so each is asked once. */
  const asked = new Set<string>();
  /** What the Jobs said they hold of each repository the last time the list was drawn. */
  const told = new Map<string, string>();
  let reportsAsked = false;

  /** Fleet's list of reported bugs, whole: it replaces what is held, and puts away a report that is gone. */
  const heardReports = (reports: ReportedBug[]) =>
    set({ reports, ...(snap.openReport !== undefined && !reports.some((one) => one.id === snap.openReport) ? { openReport: undefined } : {}) });
  const loadReports = () => {
    const reports = source.reports;
    if (reports === undefined) return;
    void reports.list().then((answer) => {
      if (answer.ok) heardReports(answer.value);
      else reportsAsked = false;
    });
  };

  const savedInto = (manifestId: string, saved: SavedQuery[]) =>
    set({ trackers: (snap.trackers ?? []).map((one) => (one.manifestId === manifestId && one.state === "read" ? { ...one, saved } : one)) });

  const run = (manifestId: string, query: string, again = false) => {
    const at = key(manifestId, query);
    if (!again && snap.runs[at] !== undefined) return;
    set({ runs: { ...snap.runs, [at]: { state: "running" } } });
    void source.run(manifestId, query).then((answer) => set({ runs: { ...snap.runs, [at]: answer.ok ? { state: "read", tickets: answer.value } : { state: "failed", said: answer.said, ...(answer.missing === undefined ? {} : { missing: answer.missing }), ...(answer.needsSetup === undefined ? {} : { needsSetup: answer.needsSetup }) } } }));
  };

  /** The same query asked again with the list kept on screen, so a card changes in place and nothing flashes. A refusal keeps what was held. */
  const refreshRun = (manifestId: string, query: string) => {
    const at = key(manifestId, query);
    if (snap.runs[at]?.state !== "read") return;
    void source.run(manifestId, query).then((answer) => {
      if (answer.ok) set({ runs: { ...snap.runs, [at]: { state: "read", tickets: answer.value } } });
    });
  };

  return {
    get: () => snap,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    /** Reads the tracker of each repository not yet read, when the tab opens or a repository joins its scope. */
    readTrackers: (manifestIds: readonly string[]) => {
      const fresh = manifestIds.filter((id) => !asked.has(id));
      if (fresh.length === 0) return;
      for (const id of fresh) asked.add(id);
      void source.trackers(fresh).then((answer) => {
        if (!answer.ok) {
          for (const id of fresh) asked.delete(id);
          return set({ failed: answer.said });
        }
        set({ trackers: [...(snap.trackers ?? []).filter((one) => !fresh.includes(one.manifestId)), ...answer.value], failed: undefined });
      });
    },
    runOf: (manifestId: string, query: string): Run | undefined => snap.runs[key(manifestId, query)],
    installOf: (manifestId: string, tool: string): Install | undefined => snap.installs[key(manifestId, tool)],
    /** Installs a tool a search found missing, or sets it up where it found no project (Fleet does both on one request), then runs that search again. **Only a press calls this**; a failure is kept with its words and offers nothing more. */
    install: (manifestId: string, tool: string, query: string) => {
      const at = key(manifestId, tool);
      if (snap.installs[at]?.state === "installing") return;
      set({ installs: { ...snap.installs, [at]: { state: "installing" } } });
      void source.install(manifestId, tool).then((answer) => {
        if (!answer.ok) return set({ installs: { ...snap.installs, [at]: { state: "failed", said: answer.said } } });
        const { [at]: _done, ...rest } = snap.installs;
        set({ installs: rest });
        run(manifestId, query, true);
      });
    },
    /**
     * Told what the Jobs the window holds say they work (`issueHoldings`). **When that differs from the last time this repository
     * was told, a Job took or gave back an issue**: the lists drawn now, and the issue open in the reader, are read again so the
     * Jobs on their cards are current. Never a timer, and a tab left and come back to is told the same.
     */
    refreshHolders: (manifestId: string, queries: readonly string[], holdings: string) => {
      const was = told.get(manifestId);
      told.set(manifestId, holdings);
      if (was === undefined || was === holdings) return;
      for (const query of new Set(queries)) refreshRun(manifestId, query);
      const open = snap.open;
      if (open === undefined || open.manifestId !== manifestId) return;
      const at = key(manifestId, open.reference);
      if (snap.reads[at]?.state !== "read") return;
      void source.read(manifestId, open.reference).then((answer) => {
        if (answer.ok) set({ reads: { ...snap.reads, [at]: { state: "read", ticket: answer.value } } });
      });
    },
    run,
    /** Reads the reported bugs once, when the tab or the Cockpit first needs them; asked again if Fleet did not answer. */
    readReports: () => {
      if (reportsAsked || source.reports === undefined) return;
      reportsAsked = true;
      loadReports();
    },
    /** Reads the reported bugs again, as a window does whenever Fleet is reached again, which is when a change could have been missed. */
    loadReports,
    /** Hears `bug_reports.changed` until the returned function is called. */
    followReports: (): (() => void) => source.reports?.follow(heardReports) ?? (() => undefined),
    /** Opening a report marks it read here, and puts any issue away. */
    openReport: (id: string | undefined) => set(id === undefined ? { openReport: undefined } : { openReport: id, open: undefined, seen: snap.seen.includes(id) ? snap.seen : [...snap.seen, id] }),
    /** Keeps a report with Fleet, and shows it kept once Fleet says so. A refusal is said on the tab, and the list is read again. */
    keepReport: async (id: string): Promise<void> => {
      set({ seen: snap.seen.includes(id) ? snap.seen : [...snap.seen, id] });
      const answer = await source.reports?.keep(id);
      if (answer === undefined) return;
      if (!answer.ok) {
        set({ failed: answer.said });
        return loadReports();
      }
      set({ reports: snap.reports.map((one) => (one.id === id ? answer.value : one)), failed: undefined });
    },
    /** Drops a report with Fleet, and puts it away once Fleet says so. A refusal is said on the tab, and the list is read again. */
    dropReport: async (id: string): Promise<void> => {
      const answer = await source.reports?.drop(id);
      if (answer === undefined) return;
      if (!answer.ok) {
        set({ failed: answer.said });
        return loadReports();
      }
      set({ reports: snap.reports.filter((one) => one.id !== id), failed: undefined, ...(snap.openReport === id ? { openReport: undefined } : {}) });
    },
    open: (manifestId: string | undefined, reference?: string) => {
      if (manifestId === undefined || reference === undefined) return set({ open: undefined, openReport: undefined });
      set({ open: { manifestId, reference }, openReport: undefined });
      const at = key(manifestId, reference);
      if (snap.reads[at] !== undefined) return;
      void source.read(manifestId, reference).then((answer) => set({ reads: { ...snap.reads, [at]: answer.ok ? { state: "read", ticket: answer.value } : { state: "failed", said: answer.said } } }));
    },
    readOf: (manifestId: string, reference: string): Read | undefined => snap.reads[key(manifestId, reference)],
    describe: () => set({ describing: { phase: "box" }, editing: undefined }),
    stopDescribing: () => set({ describing: undefined }),
    /** The model writes a query for his words in each repository's tracker, and each is run as it arrives. */
    write: (words: string, manifestIds: readonly string[]) => {
      set({ describing: { phase: "written", words, written: Object.fromEntries(manifestIds.map((id) => [id, { state: "running" }])) } });
      for (const manifestId of manifestIds) {
        void source.write(manifestId, words).then((answer) => {
          const now = snap.describing;
          if (now?.phase !== "written" || now.words !== words) return;
          set({ describing: { ...now, written: { ...now.written, [manifestId]: answer.ok ? { state: "read", query: answer.value } : { state: "failed", said: answer.said } } } });
          if (!answer.ok) return;
          // Fleet ran what the model wrote before it answered, so those issues are the list and it is not asked again.
          if (answer.value.tickets === undefined) return run(manifestId, answer.value.query);
          set({ runs: { ...snap.runs, [key(manifestId, answer.value.query)]: { state: "read", tickets: answer.value.tickets } } });
        });
      }
    },
    /** Keeps what the model wrote under `name` in each repository it was written for. Answers once every one is kept. */
    saveWritten: async (name: string): Promise<void> => {
      const now = snap.describing;
      if (now?.phase !== "written") return;
      for (const [manifestId, one] of Object.entries(now.written)) {
        if (one.state !== "read") continue;
        const answer = await source.save(manifestId, { name, words: now.words, query: one.query.query, row_fields: one.query.row_fields });
        if (!answer.ok) return set({ failed: answer.said });
        savedInto(manifestId, answer.value);
      }
      set({ describing: undefined, failed: undefined });
    },
    edit: (manifestId: string | undefined, id?: string) => set({ editing: manifestId === undefined || id === undefined ? undefined : { manifestId, id } }),
    /** A query he edited by hand is kept and run again as written. No model is called. */
    saveEdited: (manifestId: string, query: SavedQuery) => {
      void source.save(manifestId, query).then((answer) => {
        if (!answer.ok) return set({ failed: answer.said });
        savedInto(manifestId, answer.value);
        set({ editing: undefined, failed: undefined });
        run(manifestId, query.query, true);
      });
    },
    remove: (manifestId: string, id: string) => {
      void source.remove(manifestId, id).then((answer) => {
        if (!answer.ok) return set({ failed: answer.said });
        savedInto(manifestId, answer.value);
        set({ editing: undefined, failed: undefined });
      });
    },
    spend: (what: string, then: () => void) => {
      if (!snap.askFirst) return then();
      waiting = then;
      set({ spending: what });
    },
    confirmSpend: (again: boolean) => {
      const then = waiting;
      waiting = undefined;
      set({ spending: undefined, ...(again ? { askFirst: false } : {}) });
      then?.();
    },
    cancelSpend: () => {
      waiting = undefined;
      set({ spending: undefined });
    },
  };
}

const Held = createContext<IssuesStore | undefined>(undefined);

/** Hands the window its Issues store: over Fleet in the app, over a scenario in the mock. */
export function IssuesFrom({ store, children }: { store: IssuesStore | undefined; children: ReactNode }) {
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

export function useIssuesStore(): IssuesStore | undefined {
  return useContext(Held);
}

const NEVER: IssuesSnapshot = { trackers: undefined, failed: undefined, runs: {}, installs: {}, reads: {}, open: undefined, reports: [], seen: [], openReport: undefined, describing: undefined, editing: undefined, spending: undefined, askFirst: true };
const NONE = () => () => undefined;

export function useIssues(): IssuesSnapshot {
  const store = useIssuesStore();
  return useSyncExternalStore(store?.subscribe ?? NONE, store?.get ?? (() => NEVER));
}

/** Issues is left off the places and the palette where nothing serves it. */
export function issuesHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.issues];
}

/**
 * Keeps the reported bugs current while the window is open: it hears `bug_reports.changed`, and reads them whenever Fleet is
 * reached again, which is when a change could have been missed.
 */
export function useReportedBugsFollowed(connected: boolean): void {
  const store = useIssuesStore();
  useEffect(() => store?.followReports(), [store]);
  useEffect(() => {
    if (connected) store?.loadReports();
  }, [store, connected]);
}
