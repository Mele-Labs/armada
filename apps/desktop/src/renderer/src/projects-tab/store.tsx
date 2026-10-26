// The Projects tab's state for one window: his projects and their rows, the project open, the rows selected, and whether
// the link search or a new project's name field is up. **Kept on his Mac**: nothing here asks Fleet for anything, and the
// one thing that reaches a Job is the dispatch sheet, whose own act starts it.

import { createContext, useContext, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { SURFACE } from "@armada/shell";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Holder, IssueRow, Linked, LinkedPull, Look, Project, ProjectRow, RowState, TaskRow } from "./model";

export type ProjectsSnapshot = {
  projects: readonly Project[];
  /** The project open on the right, by id. */
  open: string | undefined;
  /** The rows of the open project he has selected, by id. A parent selected takes its subtasks with it. */
  selected: readonly string[];
  /** The link search is up: under the project's head, or under a task's card. */
  linking: boolean;
  /** The task the search links to, by row id; absent, what it finds joins the project as a row. */
  linkingTask: string | undefined;
  /** The task whose subtask field is up under its subtasks, by row id. */
  subtasking: string | undefined;
  /** The row deleted last, while the toast offers it back: where it stood, under its parent where it had one. */
  removed: { projectId: string; parent: string | undefined; row: ProjectRow; at: number } | undefined;
  /** The select control's look, while two are offered. */
  look: Look;
  /** The repository a new project's name field is up in, by Manifest id. */
  naming: string | undefined;
};

/** Told when he answers a done ask on a row, so the same ask on the Job is taken off. */
export type Settled = (job: string, rowId: string, done: boolean) => void;

export type ProjectsStore = ReturnType<typeof createProjectsStore>;

/** Every row of a project, each subtask after its parent. */
export const allRows = (project: Project): ProjectRow[] => project.rows.flatMap((row): ProjectRow[] => (row.kind === "task" ? [row, ...(row.subtasks ?? [])] : [row]));

/** The list a row stands in, its parent's subtasks or the project's rows, and the parent's id where it has one. */
function siblingsOf(project: Project, rowId: string): { parent: string | undefined; rows: readonly ProjectRow[] } | undefined {
  if (project.rows.some((row) => row.id === rowId)) return { parent: undefined, rows: project.rows };
  for (const row of project.rows) if (row.kind === "task" && (row.subtasks ?? []).some((one) => one.id === rowId)) return { parent: row.id, rows: row.subtasks ?? [] };
  return undefined;
}

/** A project with the list `parent` names (its own rows where undefined) changed. */
function withList(project: Project, parent: string | undefined, change: (rows: ProjectRow[]) => ProjectRow[]): Project {
  if (parent === undefined) return { ...project, rows: change([...project.rows]) };
  return { ...project, rows: project.rows.map((row) => (row.id === parent && row.kind === "task" ? { ...row, subtasks: change([...(row.subtasks ?? [])]) as TaskRow[] } : row)) };
}

/** The pull requests of each repository the search offers, by Manifest id. Only the mock serves them. */
export type PullsServed = Readonly<Record<string, readonly LinkedPull[]>>;

export function createProjectsStore(seed: readonly Project[], settled?: Settled, look: Look = "reticle", pulls: PullsServed = {}) {
  let snap: ProjectsSnapshot = { projects: structuredClone(seed) as Project[], open: undefined, selected: [], linking: false, linkingTask: undefined, subtasking: undefined, removed: undefined, look, naming: undefined };
  const listeners = new Set<() => void>();
  const set = (next: Partial<ProjectsSnapshot>) => {
    snap = { ...snap, ...next };
    for (const one of listeners) one();
  };
  let made = 0;
  const fresh = (kind: string) => `${kind}-${Date.now().toString(36)}-${(made += 1)}`;
  const edit = (projectId: string, change: (one: Project) => Project) => set({ projects: snap.projects.map((one) => (one.id === projectId ? change(one) : one)) });
  const holding = (rowId: string) => snap.projects.find((one) => allRows(one).some((row) => row.id === rowId));
  const find = (rowId: string) => snap.projects.flatMap(allRows).find((row) => row.id === rowId);
  /** Changes one row wherever it stands, a subtask included. */
  const editRow = (rowId: string, change: (row: ProjectRow) => ProjectRow) => {
    const project = holding(rowId);
    const where = project === undefined ? undefined : siblingsOf(project, rowId);
    if (project === undefined || where === undefined) return;
    edit(project.id, (one) => withList(one, where.parent, (rows) => rows.map((row) => (row.id === rowId ? change(row) : row))));
  };
  const closed = { selected: [], linking: false, linkingTask: undefined, subtasking: undefined } as const;

  return {
    get: () => snap,
    pullsOf: (manifestId: string): readonly LinkedPull[] => pulls[manifestId] ?? [],
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    open: (projectId: string | undefined) => {
      if (projectId !== undefined) edit(projectId, (one) => ({ ...one, usedAt: Date.now() }));
      set({ open: projectId, ...closed });
    },
    /** A new project in a repository. It opens only where `open` says so: the palette adds one without leaving the page he is on. */
    addProject: (manifestId: string, name: string, open = true): string => {
      const id = fresh("project");
      set({ projects: [...snap.projects, { id, manifestId, name, rows: [], usedAt: Date.now() }], naming: undefined, ...(open ? { open: id, ...closed } : {}) });
      return id;
    },
    name: (manifestId: string | undefined) => set({ naming: manifestId }),
    /** A task joins the top of the list, under the field it was written in. */
    addTask: (projectId: string, title: string) => edit(projectId, (one) => ({ ...one, usedAt: Date.now(), rows: [{ kind: "task", id: fresh("task"), title, done: false }, ...one.rows] })),
    /** A subtask joins the foot of its parent's. A subtask holds none, so one asked of a subtask is not added. */
    addSubtask: (parentId: string, title: string) => {
      const project = holding(parentId);
      if (project === undefined || !project.rows.some((row) => row.id === parentId && row.kind === "task")) return;
      edit(project.id, (one) => withList({ ...one, usedAt: Date.now() }, parentId, (rows) => [...rows, { kind: "task", id: fresh("task"), title, done: false }]));
    },
    /** The field that adds a subtask under a task's subtasks, or none. */
    subtasking: (parentId: string | undefined) => set({ subtasking: parentId }),
    /** An issue joins the list at its foot. One already linked stays where it is. */
    link: (projectId: string, issue: Omit<IssueRow, "kind" | "id">) =>
      edit(projectId, (one) => (one.rows.some((row) => row.kind === "issue" && row.reference === issue.reference) ? one : { ...one, usedAt: Date.now(), rows: [...one.rows, { kind: "issue", id: fresh("issue"), ...issue }] })),
    /** An issue on a task's card. One the task already carries stays where it is. */
    linkToTask: (rowId: string, issue: Linked) =>
      editRow(rowId, (row) => (row.kind !== "task" || (row.issues ?? []).some((one) => one.reference === issue.reference) ? row : { ...row, issues: [...(row.issues ?? []), issue] })),
    /** A Job linked to a task holds it as Dispatch would; one already holding it is let go. */
    linkJob: (rowId: string, job: Holder) => editRow(rowId, (row) => (row.kind === "task" ? { ...row, job } : row)),
    unlinkJob: (rowId: string) =>
      editRow(rowId, (row) => {
        if (row.kind !== "task") return row;
        const { job: _gone, ...rest } = row;
        void _gone;
        return rest;
      }),
    linkPull: (rowId: string, pull: LinkedPull) =>
      editRow(rowId, (row) => (row.kind !== "task" || (row.pulls ?? []).some((one) => one.number === pull.number) ? row : { ...row, pulls: [...(row.pulls ?? []), pull] })),
    unlinkPull: (rowId: string, number: number) => editRow(rowId, (row) => (row.kind === "task" ? { ...row, pulls: (row.pulls ?? []).filter((one) => one.number !== number) } : row)),
    /** A Job or a pull request linked to the project, not a task: it joins the list's foot as a task by its title, holding it. */
    linkAsRow: (projectId: string, title: string, held: { job: Holder } | { pull: LinkedPull }) =>
      edit(projectId, (one) => ({
        ...one,
        usedAt: Date.now(),
        rows: [...one.rows, { kind: "task", id: fresh("task"), title, done: false, ...("job" in held ? { job: held.job } : { pulls: [held.pull] }) }],
      })),
    /** The link search, for the project or for one task. */
    linking: (on: boolean, task?: string) => set({ linking: on, linkingTask: on ? task : undefined }),
    /** Moves a row one place up or down the list it stands in: a subtask among its parent's. */
    move: (rowId: string, by: 1 | -1) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      if (project === undefined || where === undefined) return;
      edit(project.id, (one) =>
        withList(one, where.parent, (rows) => {
          const at = rows.findIndex((row) => row.id === rowId);
          const to = at + by;
          if (at < 0 || to < 0 || to >= rows.length) return rows;
          [rows[at], rows[to]] = [rows[to]!, rows[at]!];
          return rows;
        }),
      );
    },
    /** Puts a row down before or after another of the same list, as a drag drops it. */
    place: (rowId: string, target: string, after: boolean) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      if (project === undefined || where === undefined || rowId === target) return;
      edit(project.id, (one) =>
        withList(one, where.parent, (rows) => {
          const row = rows.find((it) => it.id === rowId);
          const rest = rows.filter((it) => it.id !== rowId);
          const at = rest.findIndex((it) => it.id === target);
          if (row === undefined || at < 0) return rows;
          rest.splice(after ? at + 1 : at, 0, row);
          return rest;
        }),
      );
    },
    tick: (rowId: string) => editRow(rowId, (row) => (row.kind === "task" ? { ...row, done: !row.done } : row)),
    select: (rowId: string) => set({ selected: snap.selected.includes(rowId) ? snap.selected.filter((one) => one !== rowId) : [...snap.selected, rowId] }),
    clearSelection: () => set({ selected: [] }),
    /** A task deleted, or an issue unlinked, with no question: the toast offers it back for a moment. */
    remove: (rowId: string) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      if (project === undefined || where === undefined) return;
      const at = where.rows.findIndex((row) => row.id === rowId);
      edit(project.id, (one) => withList(one, where.parent, (rows) => rows.filter((row) => row.id !== rowId)));
      set({ selected: snap.selected.filter((one) => one !== rowId), removed: { projectId: project.id, parent: where.parent, row: where.rows[at]!, at } });
    },
    undo: () => {
      const gone = snap.removed;
      if (gone === undefined) return;
      edit(gone.projectId, (one) => withList(one, gone.parent, (rows) => [...rows.slice(0, gone.at), gone.row, ...rows.slice(gone.at)]));
      set({ removed: undefined });
    },
    forgetRemoved: () => set({ removed: undefined }),
    /** His answer to a done ask: Mark done keeps the Job on the row as the one that did it; Keep open lets the row go back to open. */
    settle: (rowId: string, done: boolean) => {
      const row = find(rowId);
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

const NEVER: ProjectsSnapshot = { projects: [], open: undefined, selected: [], linking: false, linkingTask: undefined, subtasking: undefined, removed: undefined, look: "reticle", naming: undefined };
const NONE = () => () => undefined;

export function useProjects(): ProjectsSnapshot {
  const store = useProjectsStore();
  return useSyncExternalStore(store?.subscribe ?? NONE, store?.get ?? (() => NEVER));
}

/** Projects is left off the places and the palette where nothing serves it. */
export function projectsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.projects];
}
