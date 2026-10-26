// The Projects tab's state for one window: his projects and their rows, the project open, the rows selected, and whether
// the issue search or a new project's name field is up. **Kept on his Mac**: nothing here asks Fleet for anything, and the
// one thing that reaches a Job is the dispatch sheet, whose own act starts it.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { SURFACE } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { IssueRow, Project, ProjectRow, RowState } from "./model";

export type ProjectsSnapshot = {
  projects: readonly Project[];
  /** The project open on the right, by id. */
  open: string | undefined;
  /** The rows of the open project he has selected, by id. */
  selected: readonly string[];
  /** The search of the repository's issues is up under the rows. */
  linking: boolean;
  /** The repository a new project's name field is up in, by Manifest id. */
  naming: string | undefined;
};

/** Told when he answers a done ask on a row, so the same ask on the Job is taken off. */
export type Settled = (job: string, rowId: string, done: boolean) => void;

export type ProjectsStore = ReturnType<typeof createProjectsStore>;

export function createProjectsStore(seed: readonly Project[], settled?: Settled) {
  let snap: ProjectsSnapshot = { projects: structuredClone(seed) as Project[], open: undefined, selected: [], linking: false, naming: undefined };
  const listeners = new Set<() => void>();
  const set = (next: Partial<ProjectsSnapshot>) => {
    snap = { ...snap, ...next };
    for (const one of listeners) one();
  };
  let made = 0;
  const fresh = (kind: string) => `${kind}-${Date.now().toString(36)}-${(made += 1)}`;
  const edit = (projectId: string, change: (one: Project) => Project) => set({ projects: snap.projects.map((one) => (one.id === projectId ? change(one) : one)) });
  const editRow = (rowId: string, change: (row: ProjectRow) => ProjectRow) =>
    set({ projects: snap.projects.map((one) => (one.rows.some((row) => row.id === rowId) ? { ...one, rows: one.rows.map((row) => (row.id === rowId ? change(row) : row)) } : one)) });

  return {
    get: () => snap,
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    open: (projectId: string | undefined) => {
      if (projectId !== undefined) edit(projectId, (one) => ({ ...one, usedAt: Date.now() }));
      set({ open: projectId, selected: [], linking: false });
    },
    /** A new project in a repository. It opens only where `open` says so: the palette adds one without leaving the page he is on. */
    addProject: (manifestId: string, name: string, open = true): string => {
      const id = fresh("project");
      set({ projects: [...snap.projects, { id, manifestId, name, rows: [], usedAt: Date.now() }], naming: undefined, ...(open ? { open: id, selected: [], linking: false } : {}) });
      return id;
    },
    name: (manifestId: string | undefined) => set({ naming: manifestId }),
    addTask: (projectId: string, title: string) => edit(projectId, (one) => ({ ...one, usedAt: Date.now(), rows: [...one.rows, { kind: "task", id: fresh("task"), title, done: false }] })),
    /** An issue joins the list at its foot. One already linked stays where it is. */
    link: (projectId: string, issue: Omit<IssueRow, "kind" | "id">) =>
      edit(projectId, (one) => (one.rows.some((row) => row.kind === "issue" && row.reference === issue.reference) ? one : { ...one, usedAt: Date.now(), rows: [...one.rows, { kind: "issue", id: fresh("issue"), ...issue }] })),
    linking: (on: boolean) => set({ linking: on }),
    move: (projectId: string, rowId: string, by: 1 | -1) =>
      edit(projectId, (one) => {
        const at = one.rows.findIndex((row) => row.id === rowId);
        const to = at + by;
        if (at < 0 || to < 0 || to >= one.rows.length) return one;
        const rows = [...one.rows];
        [rows[at], rows[to]] = [rows[to]!, rows[at]!];
        return { ...one, rows };
      }),
    tick: (rowId: string) => editRow(rowId, (row) => (row.kind === "task" ? { ...row, done: !row.done } : row)),
    select: (rowId: string) => set({ selected: snap.selected.includes(rowId) ? snap.selected.filter((one) => one !== rowId) : [...snap.selected, rowId] }),
    clearSelection: () => set({ selected: [] }),
    /** His answer to a done ask: Mark done keeps the Job on the row as the one that did it; Keep open lets the row go back to open. */
    settle: (rowId: string, done: boolean) => {
      const row = snap.projects.flatMap((one) => one.rows).find((one) => one.id === rowId);
      if (row?.kind !== "task" || row.job === undefined) return;
      const job = row.job.id;
      editRow(rowId, (one) => {
        if (one.kind !== "task") return one;
        if (done) return { ...one, done: true };
        const { job: _gone, ...rest } = one;
        void _gone;
        return rest;
      });
      settled?.(job, rowId, done);
    },
  };
}

/**
 * What a row's glyph says. A task held by a Job that has finished, or ended, asks whether it is done until he answers,
 * however long after; a Job still at work holds it.
 */
export function rowState(row: ProjectRow, sessions: readonly Session[]): RowState {
  if (row.kind === "issue") return row.state === "closed" ? "closed" : "open";
  if (row.done) return "done";
  if (row.job === undefined) return "open";
  const job = sessions.find((one) => one.id === row.job?.id);
  return job === undefined || job.dead !== undefined || job.standing === "done" ? "asking" : "held";
}

const Held = createContext<ProjectsStore | undefined>(undefined);

/** Hands the window its Projects store. Only the mock has one until projects are kept on the Mac. */
export function ProjectsFrom({ store, children }: { store: ProjectsStore | undefined; children: ReactNode }) {
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

export function useProjectsStore(): ProjectsStore | undefined {
  return useContext(Held);
}

const NEVER: ProjectsSnapshot = { projects: [], open: undefined, selected: [], linking: false, naming: undefined };
const NONE = () => () => undefined;

export function useProjects(): ProjectsSnapshot {
  const store = useProjectsStore();
  return useSyncExternalStore(store?.subscribe ?? NONE, store?.get ?? (() => NEVER));
}

/** Projects is left off the places and the palette where nothing serves it. */
export function projectsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.projects];
}
