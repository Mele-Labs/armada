// The Projects tab's state for one window: his projects and their rows, the project open, the rows selected, and whether
// a new project's name field is up. Each repository also has its own tasks, a project marked `own` that the store keeps
// ready for it and Fleet makes on its first task. **Kept in Fleet**: every write goes to its route and the repository's
// projects come back whole, as `projects.changed` carries them too. A change that only moves what is already drawn (a tick, a
// move, a delete, a rename) is drawn at once and then replaced by what Fleet answers; one that makes a row (a task, a link)
// waits for the answer, since Fleet gives the row its id. Writes go one after another, so a key pressed twice acts twice.

import { createContext, useContext, useEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { Projects, RepositorySummary } from "@armada/protocol";
import { SURFACE, jobIdOf } from "@armada/shell";

import type { ProjectsActed, ProjectsApi } from "../../../shared/api/projects";
import { ownId, type Holder, type Linked, type LinkedPull, type Project, type ProjectRow, type RowState, type TaskLinks, type TaskRow } from "./model";
import { projectsOf } from "./wire";

export type ProjectsSnapshot = {
  projects: readonly Project[];
  /** The project open on the right, by id. */
  open: string | undefined;
  /** The rows of the open project he has selected, by id. A parent selected takes its subtasks with it. */
  selected: readonly string[];
  /** The card last clicked or selected, by row id: where a ⇧-click's range starts. */
  anchor: string | undefined;
  /** The task whose subtask field is up under its subtasks, by row id. */
  subtasking: string | undefined;
  /** What was deleted last, while the toast offers it back: a row where it stood, under its parent where it had one, or a project. */
  removed: { kind: "row"; projectId: string; parent: string | undefined; row: ProjectRow; at: number } | { kind: "project"; project: Project; at: number } | undefined;
  /** The task whose title is being edited in place, by row id. */
  editing: string | undefined;
  /** The project whose name is being edited in the list, by id. */
  renaming: string | undefined;
  /** The repository a new project's name field is up in, by Manifest id. */
  naming: string | undefined;
};

export type ProjectsStore = ReturnType<typeof createProjectsStore>;

/** What the store reaches Fleet through: `window.armada` in the app, the mock Fleet's in a walk. */
export type ProjectsSource = ProjectsApi;

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

/** The pull requests of each repository the search offers, by Manifest id. Only the mock serves them; the app offers the numbered ones. */
export type PullsServed = Readonly<Record<string, readonly LinkedPull[]>>;

const idsOf = (wire: Projects | undefined, projectId: string): Set<string> => new Set(wire?.projects.find((one) => one.id === projectId)?.rows.map((row) => row.id));

export function createProjectsStore(source: ProjectsSource, pulls: PullsServed = {}) {
  let snap: ProjectsSnapshot = { projects: [], open: undefined, selected: [], anchor: undefined, subtasking: undefined, removed: undefined, editing: undefined, renaming: undefined, naming: undefined };
  const listeners = new Set<() => void>();
  const set = (next: Partial<ProjectsSnapshot>) => {
    snap = { ...snap, ...next };
    for (const one of listeners) one();
  };
  /** What Fleet last said of each repository, whole, and when each project was last used on this Mac. */
  const wire = new Map<string, Projects>();
  const used = new Map<string, number>();
  /** Writes in flight. While any is, what Fleet says waits: the drawing already shows where they end up. */
  let pending = 0;
  let tail: Promise<void> = Promise.resolve();

  const rebuild = () => {
    // Every repository read has its own tasks ready, though Fleet makes the list only when the first is written.
    const projects = [...wire.values()].flatMap((one) => {
      const listed = projectsOf(one, used);
      const own = ownId(one.manifest_id);
      return listed.some((project) => project.id === own) ? listed : [...listed, { id: own, manifestId: one.manifest_id, name: "", rows: [], usedAt: used.get(own) ?? 0, own: true as const }];
    });
    const rows = new Set(projects.flatMap((one) => allRows(one).map((row) => row.id)));
    set({ projects, selected: snap.selected.filter((one) => rows.has(one)) });
  };
  const heard = (projects: Projects) => {
    wire.set(projects.manifest_id, projects);
    if (pending === 0) rebuild();
  };
  const reread = async (manifestId: string) => {
    const read = await source.listProjects(manifestId);
    if (read.ok) wire.set(read.value.manifest_id, read.value);
  };
  /**
   * One write, after those before it. `run` makes the calls and answers the last of them; a refusal is not worded here, since the
   * drawing was either never changed or is put right by reading what Fleet holds. Resolves to the projects as answered, drawn.
   */
  const queue = (manifestId: string, run: () => Promise<ProjectsActed | undefined>): Promise<Projects | undefined> => {
    pending += 1;
    const result = tail
      .then(async () => {
        const answer = await run().catch(() => undefined);
        if (answer?.ok === true) return void wire.set(answer.value.manifest_id, answer.value);
        await reread(manifestId);
      })
      .finally(() => {
        pending -= 1;
        if (pending === 0) rebuild();
      })
      .then(() => wire.get(manifestId));
    tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  };

  const edit = (projectId: string, change: (one: Project) => Project) => set({ projects: snap.projects.map((one) => (one.id === projectId ? change(one) : one)) });
  const holding = (rowId: string) => snap.projects.find((one) => allRows(one).some((row) => row.id === rowId));
  const find = (rowId: string) => snap.projects.flatMap(allRows).find((row) => row.id === rowId);
  const projectOf = (projectId: string) => snap.projects.find((one) => one.id === projectId);
  const touch = (projectId: string) => {
    used.set(projectId, Date.now());
    edit(projectId, (one) => ({ ...one, usedAt: used.get(projectId)! }));
  };
  /** Changes one row wherever it stands, a subtask included. */
  const editRow = (rowId: string, change: (row: ProjectRow) => ProjectRow) => {
    const project = holding(rowId);
    const where = project === undefined ? undefined : siblingsOf(project, rowId);
    if (project === undefined || where === undefined) return;
    edit(project.id, (one) => withList(one, where.parent, (rows) => rows.map((row) => (row.id === rowId ? change(row) : row))));
  };
  const closed = { selected: [], anchor: undefined, subtasking: undefined, editing: undefined } as const;
  /** The rows selected that stand in this row's repository. A Job runs in one, so a pick in another starts a selection of its own. */
  const inRepository = (rowId: string): readonly string[] => {
    const manifestId = holding(rowId)?.manifestId;
    return snap.selected.filter((one) => holding(one)?.manifestId === manifestId);
  };

  /** Where a row stands after a change drawn: its place among the rows of the list it is in, for the route that puts it there. */
  const placed = (rowId: string): { project: Project; position: number } | undefined => {
    const project = holding(rowId);
    const where = project === undefined ? undefined : siblingsOf(project, rowId);
    return project === undefined || where === undefined ? undefined : { project, position: where.rows.findIndex((row) => row.id === rowId) };
  };

  /** The links a task was written with, put on the row Fleet made. Stops at the first refusal. */
  const linking = async (manifestId: string, projectId: string, rowId: string, links: TaskLinks, last: ProjectsActed): Promise<ProjectsActed> => {
    let answer = last;
    for (const issue of links.issues ?? []) {
      answer = await source.linkTaskIssue(manifestId, projectId, rowId, { issue: issue.reference });
      if (!answer.ok) return answer;
    }
    for (const pull of links.pulls ?? []) {
      answer = await source.linkTaskPull(manifestId, projectId, rowId, { number: pull.number });
      if (!answer.ok) return answer;
    }
    if (links.job !== undefined) answer = await source.linkTaskJob(manifestId, projectId, rowId, { session_id: links.job.id });
    return answer;
  };
  /** The row a write made: the one in the answer that was not there before. */
  const made = (answer: Projects, projectId: string, before: ReadonlySet<string>): string | undefined => answer.projects.find((one) => one.id === projectId)?.rows.find((row) => !before.has(row.id))?.id;

  /** Puts a row he deleted back in Fleet: its issues and pull requests, its subtasks, and its place. A Job that held it does not hold it again. */
  const recreate = async (manifestId: string, projectId: string, parent: string | undefined, row: ProjectRow, at: number | undefined, from: Projects | undefined): Promise<ProjectsActed> => {
    const before = idsOf(from, projectId);
    let answer: ProjectsActed;
    if (row.kind === "issue") answer = await source.linkProjectIssue(manifestId, projectId, { issue: row.reference });
    else if (parent === undefined) answer = await source.addProjectTask(manifestId, projectId, { title: row.title, notes: row.notes ?? "" });
    else answer = await source.addProjectSubtask(manifestId, projectId, parent, { title: row.title, notes: row.notes ?? "" });
    if (!answer.ok) return answer;
    const id = made(answer.value, projectId, before);
    if (id === undefined) return answer;
    if (row.kind === "task") {
      answer = await linking(manifestId, projectId, id, { ...(row.issues === undefined ? {} : { issues: row.issues }), ...(row.pulls === undefined ? {} : { pulls: row.pulls }) }, answer);
      for (const sub of row.subtasks ?? []) {
        if (!answer.ok) return answer;
        answer = await recreate(manifestId, projectId, id, sub, undefined, answer.value);
      }
      if (answer.ok && row.done) answer = await source.tickProjectTask(manifestId, projectId, id, { done: true });
    }
    if (answer.ok && at !== undefined) answer = await source.moveProjectRow(manifestId, projectId, id, { position: at });
    return answer;
  };

  return {
    get: () => snap,
    pullsOf: (manifestId: string): readonly LinkedPull[] => pulls[manifestId] ?? [],
    subscribe: (listener: () => void) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    /** Hears `projects.changed` until the returned function is called. */
    follow: () => source.onProjectsChanged(heard),
    /** Reads these repositories' projects, as a window does when it opens and whenever Fleet is reached again. */
    load: (manifestIds: readonly string[]) => {
      for (const id of manifestIds) {
        void source.listProjects(id).then((read) => {
          if (read.ok) heard(read.value);
        });
      }
    },
    open: (projectId: string | undefined) => {
      if (projectId !== undefined) touch(projectId);
      set({ open: projectId, ...closed });
    },
    /** A new project in a repository. It opens only where `open` says so: the palette adds one without leaving the page he is on. */
    addProject: async (manifestId: string, name: string, open = true): Promise<string | undefined> => {
      set({ naming: undefined });
      let before = new Set<string>();
      const answer = await queue(manifestId, async () => {
        if (!wire.has(manifestId)) await reread(manifestId);
        before = new Set(wire.get(manifestId)?.projects.map((one) => one.id));
        return source.addProject(manifestId, { name });
      });
      const id = answer?.projects.find((one) => !before.has(one.id))?.id;
      if (id !== undefined && open) {
        used.set(id, Date.now());
        set({ open: id, ...closed });
        rebuild();
      }
      return id;
    },
    name: (manifestId: string | undefined) => set({ naming: manifestId }),
    /** A task joins the top of the list, under the field it was written in. */
    addTask: (projectId: string, title: string, links: TaskLinks = {}) => {
      const project = projectOf(projectId);
      if (project === undefined) return;
      const manifestId = project.manifestId;
      touch(projectId);
      void queue(manifestId, async () => {
        const before = idsOf(wire.get(manifestId), projectId);
        const added = await source.addProjectTask(manifestId, projectId, { title });
        if (!added.ok) return added;
        const id = made(added.value, projectId, before);
        if (id === undefined) return added;
        const moved = await source.moveProjectRow(manifestId, projectId, id, { position: 0 });
        return moved.ok ? linking(manifestId, projectId, id, links, moved) : moved;
      });
    },
    /** A subtask joins the foot of its parent's. A subtask holds none, so one asked of a subtask is not added. */
    addSubtask: (parentId: string, title: string, links: TaskLinks = {}) => {
      const project = holding(parentId);
      if (project === undefined || !project.rows.some((row) => row.id === parentId && row.kind === "task")) return;
      const manifestId = project.manifestId;
      touch(project.id);
      void queue(manifestId, async () => {
        const before = idsOf(wire.get(manifestId), project.id);
        const added = await source.addProjectSubtask(manifestId, project.id, parentId, { title });
        if (!added.ok) return added;
        const id = made(added.value, project.id, before);
        return id === undefined ? added : linking(manifestId, project.id, id, links, added);
      });
    },
    /** The field that adds a subtask under a task's subtasks, or none. */
    subtasking: (parentId: string | undefined) => set({ subtasking: parentId }),
    /** An issue on a task's card. One the task already carries stays where it is. */
    linkToTask: (rowId: string, issue: Linked) => {
      const project = holding(rowId);
      const row = find(rowId);
      if (project === undefined || row?.kind !== "task" || (row.issues ?? []).some((one) => one.reference === issue.reference)) return;
      void queue(project.manifestId, () => source.linkTaskIssue(project.manifestId, project.id, rowId, { issue: issue.reference }));
    },
    /** A Job linked to a task holds it as Dispatch would; one already holding it is let go. */
    linkJob: (rowId: string, job: Holder) => {
      const project = holding(rowId);
      if (project !== undefined) void queue(project.manifestId, () => source.linkTaskJob(project.manifestId, project.id, rowId, { session_id: job.id }));
    },
    unlinkJob: (rowId: string) => {
      const project = holding(rowId);
      if (project === undefined) return;
      editRow(rowId, (row) => {
        if (row.kind !== "task") return row;
        const { job: _gone, ...rest } = row;
        void _gone;
        return rest;
      });
      void queue(project.manifestId, () => source.unlinkTaskJob(project.manifestId, project.id, rowId));
    },
    linkPull: (rowId: string, pull: LinkedPull) => {
      const project = holding(rowId);
      const row = find(rowId);
      if (project === undefined || row?.kind !== "task" || (row.pulls ?? []).some((one) => one.number === pull.number)) return;
      void queue(project.manifestId, () => source.linkTaskPull(project.manifestId, project.id, rowId, { number: pull.number }));
    },
    unlinkPull: (rowId: string, number: number) => {
      const project = holding(rowId);
      if (project === undefined) return;
      editRow(rowId, (row) => (row.kind === "task" ? { ...row, pulls: (row.pulls ?? []).filter((one) => one.number !== number) } : row));
      void queue(project.manifestId, () => source.unlinkTaskPull(project.manifestId, project.id, rowId, { number }));
    },
    /** Moves a row one place up or down the list it stands in: a subtask among its parent's. */
    move: (rowId: string, by: 1 | -1) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      if (project === undefined || where === undefined) return;
      const at = where.rows.findIndex((row) => row.id === rowId);
      if (at < 0 || at + by < 0 || at + by >= where.rows.length) return;
      edit(project.id, (one) =>
        withList(one, where.parent, (rows) => {
          [rows[at], rows[at + by]] = [rows[at + by]!, rows[at]!];
          return rows;
        }),
      );
      void queue(project.manifestId, () => source.moveProjectRow(project.manifestId, project.id, rowId, { position: at + by }));
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
      const now = placed(rowId);
      if (now !== undefined) void queue(project.manifestId, () => source.moveProjectRow(project.manifestId, project.id, rowId, { position: now.position }));
    },
    /** Done, or open again. On a row Fleet asks about, it answers the ask. */
    tick: (rowId: string) => {
      const project = holding(rowId);
      const row = find(rowId);
      if (project === undefined || row?.kind !== "task") return;
      const done = !row.done;
      editRow(rowId, (one) => {
        if (one.kind !== "task") return one;
        const { asked: _asked, ...rest } = one;
        void _asked;
        return { ...rest, done };
      });
      void queue(project.manifestId, () => source.tickProjectTask(project.manifestId, project.id, rowId, { done }));
    },
    select: (rowId: string) => {
      const kept = inRepository(rowId);
      set({ selected: kept.includes(rowId) ? kept.filter((one) => one !== rowId) : [...kept, rowId], anchor: rowId });
    },
    /** The cards from the one last clicked to this one join the selection, when both stand in the same list; otherwise this one alone does. */
    selectTo: (rowId: string) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      const from = where?.rows.findIndex((row) => row.id === snap.anchor) ?? -1;
      const to = where?.rows.findIndex((row) => row.id === rowId) ?? -1;
      const kept = inRepository(rowId);
      if (where === undefined || from < 0 || to < 0) return set({ selected: kept.includes(rowId) ? kept : [...kept, rowId], anchor: rowId });
      const between = where.rows.slice(Math.min(from, to), Math.max(from, to) + 1).map((row) => row.id);
      set({ selected: [...kept, ...between.filter((one) => !kept.includes(one))], anchor: rowId });
    },
    /** A click that selects nothing still marks where a range will start. */
    anchorAt: (rowId: string) => set({ anchor: rowId }),
    clearSelection: () => set({ selected: [], anchor: undefined }),
    /** A task deleted, or an issue unlinked, with no question: the toast offers it back for a moment. */
    remove: (rowId: string) => {
      const project = holding(rowId);
      const where = project === undefined ? undefined : siblingsOf(project, rowId);
      if (project === undefined || where === undefined) return;
      const at = where.rows.findIndex((row) => row.id === rowId);
      edit(project.id, (one) => withList(one, where.parent, (rows) => rows.filter((row) => row.id !== rowId)));
      set({ selected: snap.selected.filter((one) => one !== rowId), removed: { kind: "row", projectId: project.id, parent: where.parent, row: where.rows[at]!, at } });
      void queue(project.manifestId, () => source.removeProjectRow(project.manifestId, project.id, rowId));
    },
    /** A project deleted from the list, with no question: the toast offers it back, rows and all. */
    removeProject: (projectId: string) => {
      const at = snap.projects.findIndex((one) => one.id === projectId);
      if (at < 0 || snap.projects[at]!.own === true) return;
      const project = snap.projects[at]!;
      set({ projects: snap.projects.filter((one) => one.id !== projectId), removed: { kind: "project", project, at }, renaming: undefined, ...(snap.open === projectId ? { open: undefined, ...closed } : {}) });
      void queue(project.manifestId, () => source.deleteProject(project.manifestId, projectId));
    },
    /** What he deleted comes back as new rows in Fleet, a project at the foot of its repository's, since projects keep no place of their own. */
    undo: () => {
      const gone = snap.removed;
      if (gone === undefined) return;
      set({ removed: undefined });
      if (gone.kind === "project") {
        const { manifestId, name, rows } = gone.project;
        void queue(manifestId, async () => {
          const before = new Set(wire.get(manifestId)?.projects.map((one) => one.id));
          let answer = await source.addProject(manifestId, { name });
          if (!answer.ok) return answer;
          const id = answer.value.projects.find((one) => !before.has(one.id))?.id;
          if (id === undefined) return answer;
          for (const row of rows) {
            if (!answer.ok) return answer;
            answer = await recreate(manifestId, id, undefined, row, undefined, answer.value);
          }
          return answer;
        });
        return;
      }
      const project = projectOf(gone.projectId);
      if (project === undefined) return;
      edit(gone.projectId, (one) => withList(one, gone.parent, (rows) => [...rows.slice(0, gone.at), gone.row, ...rows.slice(gone.at)]));
      void queue(project.manifestId, () => recreate(project.manifestId, gone.projectId, gone.parent, gone.row, gone.at, wire.get(project.manifestId)));
    },
    /** A task's title edited in place, or none. */
    editing: (rowId: string | undefined) => set({ editing: rowId }),
    retitle: (rowId: string, title: string) => {
      const project = holding(rowId);
      const row = find(rowId);
      set({ editing: undefined });
      if (project === undefined || row?.kind !== "task") return;
      editRow(rowId, (one) => (one.kind === "task" ? { ...one, title } : one));
      void queue(project.manifestId, () => source.editProjectTask(project.manifestId, project.id, rowId, { title, notes: row.notes ?? "" }));
    },
    /** A project's name edited in the list, or none. */
    renaming: (projectId: string | undefined) => set({ renaming: projectId }),
    renameProject: (projectId: string, name: string) => {
      const project = projectOf(projectId);
      set({ renaming: undefined });
      if (project === undefined || project.own === true) return;
      edit(projectId, (one) => ({ ...one, name }));
      void queue(project.manifestId, () => source.renameProject(project.manifestId, projectId, { name }));
    },
    forgetRemoved: () => set({ removed: undefined }),
    /** His answer to a done ask: Mark done, or Keep open. Fleet takes the same ask off the Job that raised it. */
    settle: (rowId: string, done: boolean) => {
      const project = holding(rowId);
      const row = find(rowId);
      if (project === undefined || row?.kind !== "task" || row.asked !== true) return;
      editRow(rowId, (one) => {
        if (one.kind !== "task") return one;
        const { asked: _asked, job: _job, ...rest } = one;
        void _asked;
        void _job;
        return { ...rest, done };
      });
      void queue(project.manifestId, () => source.tickProjectTask(project.manifestId, project.id, rowId, { done }));
    },
  };
}

/** What a row's glyph says. A task Fleet asks about is asking, until he answers, however long after; one a Job holds is held. */
export function rowState(row: ProjectRow): RowState {
  if (row.kind === "issue") return row.state === "closed" ? "closed" : "open";
  if (row.done) return "done";
  if (row.asked === true) return "asking";
  return row.job === undefined ? "open" : "held";
}

const Held = createContext<ProjectsStore | undefined>(undefined);

/** Hands the window its Projects store. A window with none has no Projects tab. */
export function ProjectsFrom({ store, children }: { store: ProjectsStore | undefined; children: ReactNode }) {
  return <Held.Provider value={store}>{children}</Held.Provider>;
}

export function useProjectsStore(): ProjectsStore | undefined {
  return useContext(Held);
}

const NEVER: ProjectsSnapshot = { projects: [], open: undefined, selected: [], anchor: undefined, subtasking: undefined, removed: undefined, editing: undefined, renaming: undefined, naming: undefined };
const NONE = () => () => undefined;

export function useProjects(): ProjectsSnapshot {
  const store = useProjectsStore();
  return useSyncExternalStore(store?.subscribe ?? NONE, store?.get ?? (() => NEVER));
}

/**
 * Keeps the store current while the window is open: it hears `projects.changed`, and reads every repository's projects when
 * the list of repositories changes and whenever Fleet is reached again, which is when a change could have been missed.
 */
export function useProjectsFollowed(repositories: readonly RepositorySummary[], connected: boolean): void {
  const store = useProjectsStore();
  const ids = repositories.flatMap((one) => jobIdOf(one) ?? []).join("\n");
  useEffect(() => store?.follow(), [store]);
  useEffect(() => {
    if (connected && ids !== "") store?.load(ids.split("\n"));
  }, [store, ids, connected]);
}

/** Projects is left off the places and the palette where nothing serves it. */
export function projectsHidden(served: boolean): readonly string[] {
  return served ? [] : [SURFACE.projects];
}
