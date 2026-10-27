// The Projects tab's state for one window: his projects, the tasks in no project (`loose`), the project open, the rows selected,
// and whether a new project's name field is up. **A project spans repositories**: each row names its own, so a project is merged
// here from what each repository's Fleet lists under one id. A repository's tasks in no project are the wire's project
// `ownWire(manifest)`, read here as loose tasks. **Kept in Fleet**: every write goes to its route and the repository's projects
// come back whole, as `projects.changed` carries them too. A change that only moves what is already drawn (a tick, a move, a
// delete, a rename) is drawn at once and then replaced by what Fleet answers; one that makes a row (a task, a link) waits for the
// answer, since Fleet gives the row its id. Writes go one after another, so a key pressed twice acts twice.

import { createContext, useContext, useEffect, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { ProjectRowRef, Projects, RepositorySummary } from "@armada/protocol";
import { SURFACE, jobIdOf } from "@armada/shell";

import type { ProjectsActed, ProjectsApi } from "../../../shared/api/projects";
import type { Holder, Linked, LinkedPull, Project, ProjectRow, RowState, TaskLinks, TaskRow } from "./model";
import { looseOf, ownWire, projectsOf } from "./wire";

export type ProjectsSnapshot = {
  projects: readonly Project[];
  /** The tasks in no project, of every repository. */
  loose: readonly ProjectRow[];
  /** The project open on the right, by id. */
  open: string | undefined;
  /** The rows he has selected, by id, in any list and any repository. A parent selected takes its subtasks with it. */
  selected: readonly string[];
  /** The card last clicked or selected, by row id: where a ⇧-click's range starts. */
  anchor: string | undefined;
  /** The task whose subtask field is up under its subtasks, by row id. */
  subtasking: string | undefined;
  /** What was deleted last, while the toast offers it back: a row where it stood, under its parent where it had one, or a project. `list` is the project's id, empty for a loose task. */
  removed: { kind: "row"; list: string; parent: string | undefined; row: ProjectRow; at: number; slice: number } | { kind: "project"; project: Project; homes: string[]; at: number } | undefined;
  /** The task whose title is being edited in place, by row id. */
  editing: string | undefined;
  /** The project whose name is being edited in the list, by id. */
  renaming: string | undefined;
  /** The new project's name field is up in the list. */
  naming: boolean;
  /** The repository the last task was added to, by Manifest id: the composer offers it first. */
  lastRepo: string | undefined;
  /** Done tasks are drawn. Off, they are hidden in every list. */
  showDone: boolean;
  /** The row the palette chose, by id, until the page has brought its card into view. */
  reveal: string | undefined;
};

export type ProjectsStore = ReturnType<typeof createProjectsStore>;

/** What the store reaches Fleet through: `window.armada` in the app, the mock Fleet's in a walk. */
export type ProjectsSource = ProjectsApi;

/** The key of the tasks in no project, among the lists a row can stand in. */
const LOOSE = "";

/** Every row of a list, each subtask after its parent. */
export const allRows = (list: { rows: readonly ProjectRow[] }): ProjectRow[] => list.rows.flatMap((row): ProjectRow[] => (row.kind === "task" ? [row, ...(row.subtasks ?? [])] : [row]));

/** The task a subtask stands under, or none where the row is not a subtask. */
export const parentOf = (snap: Pick<ProjectsSnapshot, "projects" | "loose">, rowId: string): TaskRow | undefined =>
  [...snap.loose, ...snap.projects.flatMap((one) => one.rows)].find((row): row is TaskRow => row.kind === "task" && (row.subtasks ?? []).some((one) => one.id === rowId));

/** A row done: a task ticked, or an issue its tracker closed. */
export const isDone = (row: ProjectRow): boolean => rowState(row) === "done" || rowState(row) === "closed";

/** The list a row stands in, its parent's subtasks or the list's rows, and the parent's id where it has one. */
function siblingsOf(rows: readonly ProjectRow[], rowId: string): { parent: string | undefined; rows: readonly ProjectRow[] } | undefined {
  if (rows.some((row) => row.id === rowId)) return { parent: undefined, rows };
  for (const row of rows) if (row.kind === "task" && (row.subtasks ?? []).some((one) => one.id === rowId)) return { parent: row.id, rows: row.subtasks ?? [] };
  return undefined;
}

/** A list's rows with the list `parent` names (its own rows where undefined) changed. */
function withList(rows: readonly ProjectRow[], parent: string | undefined, change: (rows: ProjectRow[]) => ProjectRow[]): ProjectRow[] {
  if (parent === undefined) return change([...rows]);
  return rows.map((row) => (row.id === parent && row.kind === "task" ? { ...row, subtasks: change([...(row.subtasks ?? [])]) as TaskRow[] } : row));
}

/** Where a row stands among those of its own repository in a list: the place Fleet knows it by. */
const sliceOf = (rows: readonly ProjectRow[], row: ProjectRow): number => rows.filter((one) => one.manifestId === row.manifestId).findIndex((one) => one.id === row.id);

/** The pull requests of each repository the search offers, by Manifest id. Only the mock serves them; the app offers the numbered ones. */
export type PullsServed = Readonly<Record<string, readonly LinkedPull[]>>;

const idsOf = (wire: Projects | undefined, projectId: string): Set<string> => new Set(wire?.projects.find((one) => one.id === projectId)?.rows.map((row) => row.id));

export function createProjectsStore(source: ProjectsSource, pulls: PullsServed = {}) {
  let snap: ProjectsSnapshot = { projects: [], loose: [], open: undefined, selected: [], anchor: undefined, subtasking: undefined, removed: undefined, editing: undefined, renaming: undefined, naming: false, lastRepo: undefined, showDone: false, reveal: undefined };
  const listeners = new Set<() => void>();
  const set = (next: Partial<ProjectsSnapshot>) => {
    snap = { ...snap, ...next };
    for (const one of listeners) one();
  };
  /** What Fleet last said of each repository, whole, and when each project was last used on this Mac. */
  const wire = new Map<string, Projects>();
  const used = new Map<string, number>();
  /** The order of a list that spans repositories, by its key: each repository's Fleet orders only its own rows. */
  const order = new Map<string, string[]>();
  /** Writes in flight. While any is, what Fleet says waits: the drawing already shows where they end up. */
  let pending = 0;
  let tail: Promise<void> = Promise.resolve();

  /** A list's rows in the order last shown where they span repositories, and in Fleet's own where they do not. New rows come first. */
  const ordered = (key: string, rows: ProjectRow[]): ProjectRow[] => {
    if (new Set(rows.map((one) => one.manifestId)).size < 2) {
      order.delete(key);
      return rows;
    }
    const kept = order.get(key) ?? [];
    const at = (row: ProjectRow) => kept.indexOf(row.id);
    const out = [...rows.filter((row) => at(row) < 0), ...rows.filter((row) => at(row) >= 0).sort((a, b) => at(a) - at(b))];
    order.set(key, out.map((row) => row.id));
    return out;
  };

  const rebuild = () => {
    const merged = new Map<string, Project>();
    const loose: ProjectRow[] = [];
    for (const manifestId of [...wire.keys()].sort()) {
      const one = wire.get(manifestId)!;
      for (const project of projectsOf(one, used)) {
        const was = merged.get(project.id);
        merged.set(project.id, was === undefined ? project : { ...was, rows: [...was.rows, ...project.rows], usedAt: Math.max(was.usedAt, project.usedAt) });
      }
      loose.push(...looseOf(one));
    }
    const projects = [...merged.values()].map((one) => ({ ...one, rows: ordered(one.id, one.rows) }));
    const rows = new Set([...allRows({ rows: loose }), ...projects.flatMap(allRows)].map((row) => row.id));
    set({ projects, loose: ordered(LOOSE, loose), selected: snap.selected.filter((one) => rows.has(one)) });
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

  const projectOf = (projectId: string) => snap.projects.find((one) => one.id === projectId);
  const lists = (): { key: string; rows: readonly ProjectRow[] }[] => [{ key: LOOSE, rows: snap.loose }, ...snap.projects.map((one) => ({ key: one.id, rows: one.rows }))];
  const rowsIn = (key: string): readonly ProjectRow[] => lists().find((one) => one.key === key)?.rows ?? [];
  /** The list a row stands in: a project's id, or the empty key for a task in no project. */
  const holding = (rowId: string): string | undefined => lists().find((one) => allRows(one).some((row) => row.id === rowId))?.key;
  const find = (rowId: string) => lists().flatMap(allRows).find((row) => row.id === rowId);
  /** Where Fleet knows a row: its repository, and the project it is under there. */
  const homeOf = (rowId: string): { key: string; row: ProjectRow; manifestId: string; wireId: string } | undefined => {
    const key = holding(rowId);
    const row = find(rowId);
    return key === undefined || row === undefined ? undefined : { key, row, manifestId: row.manifestId, wireId: key === LOOSE ? ownWire(row.manifestId) : key };
  };
  /** The repositories whose Fleet holds this project. */
  const homes = (projectId: string): string[] => [...wire.values()].filter((one) => one.projects.some((project) => project.id === projectId)).map((one) => one.manifest_id);

  /** A list changed where it stands. */
  const edit = (key: string, change: (rows: ProjectRow[]) => ProjectRow[]) => {
    if (key === LOOSE) return set({ loose: change([...snap.loose]) });
    set({ projects: snap.projects.map((one) => (one.id === key ? { ...one, rows: change([...one.rows]) } : one)) });
  };
  const remember = (key: string) => {
    if (new Set(rowsIn(key).map((one) => one.manifestId)).size > 1) order.set(key, rowsIn(key).map((one) => one.id));
  };
  const touch = (projectId: string) => {
    used.set(projectId, Date.now());
    set({ projects: snap.projects.map((one) => (one.id === projectId ? { ...one, usedAt: used.get(projectId)! } : one)) });
  };
  /** Changes one row wherever it stands, a subtask included. */
  const editRow = (rowId: string, change: (row: ProjectRow) => ProjectRow) => {
    const key = holding(rowId);
    const where = key === undefined ? undefined : siblingsOf(rowsIn(key), rowId);
    if (key === undefined || where === undefined) return;
    edit(key, (rows) => withList(rows, where.parent, (list) => list.map((row) => (row.id === rowId ? change(row) : row))));
  };
  const closed = { selected: [], anchor: undefined, subtasking: undefined, editing: undefined } as const;

  /** Where a row stands after a change drawn: its place among the rows of its repository in the list it is in, for the route that puts it there. */
  const placed = (rowId: string): number | undefined => {
    const key = holding(rowId);
    const where = key === undefined ? undefined : siblingsOf(rowsIn(key), rowId);
    const row = find(rowId);
    return where === undefined || row === undefined ? undefined : sliceOf(where.rows, row);
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
    /** The rows `rowId` and its kin are asked for by Fleet: the project it is under there, for a Job started with them. */
    refOf: (rowId: string): ProjectRowRef | undefined => {
      const home = homeOf(rowId);
      return home === undefined ? undefined : { project: home.wireId, row: rowId };
    },
    open: (projectId: string | undefined) => {
      if (projectId !== undefined) touch(projectId);
      set({ open: projectId, ...closed });
    },
    /**
     * Opens the list a row stands in, for the palette: its project, or All tasks for a task in no project, with done shown where
     * the row or its parent is done. The page brings the card into view and calls `shownRow`.
     */
    showRow: (rowId: string) => {
      const key = holding(rowId);
      const row = find(rowId);
      if (key === undefined || row === undefined) return;
      const parent = parentOf(snap, rowId);
      if (key !== LOOSE) touch(key);
      set({ open: key === LOOSE ? undefined : key, showDone: isDone(row) || (parent !== undefined && isDone(parent)) ? true : snap.showDone, reveal: rowId, ...closed });
    },
    shownRow: () => set({ reveal: undefined }),
    /** Done tasks drawn, or hidden. */
    showDone: (shown: boolean) =>
      set({
        showDone: shown,
        selected: shown
          ? snap.selected
          : snap.selected.filter((one) => {
              const row = find(one);
              return row === undefined || !isDone(row);
            }),
      }),
    /** A new project, kept in the repository `home`, which only Fleet's storage needs: the project is his. It opens only where `open` says so: the palette adds one without leaving the page he is on. */
    addProject: async (name: string, home: string, open = true): Promise<string | undefined> => {
      set({ naming: false });
      let before = new Set<string>();
      const answer = await queue(home, async () => {
        if (!wire.has(home)) await reread(home);
        before = new Set(wire.get(home)?.projects.map((one) => one.id));
        return source.addProject(home, { name });
      });
      const id = answer?.projects.find((one) => !before.has(one.id))?.id;
      if (id !== undefined && open) {
        used.set(id, Date.now());
        set({ open: id, ...closed });
        rebuild();
      }
      return id;
    },
    name: (naming: boolean) => set({ naming }),
    /** A task joins the top of the list, in the repository `manifestId` and under the project, or none. */
    addTask: (manifestId: string, projectId: string | undefined, title: string, links: TaskLinks = {}) => {
      const wireId = projectId ?? ownWire(manifestId);
      if (projectId !== undefined) touch(projectId);
      set({ lastRepo: manifestId });
      void queue(manifestId, async () => {
        const before = idsOf(wire.get(manifestId), wireId);
        const added = await source.addProjectTask(manifestId, wireId, { title });
        if (!added.ok) return added;
        const id = made(added.value, wireId, before);
        if (id === undefined) return added;
        const moved = await source.moveProjectRow(manifestId, wireId, id, { position: 0 });
        return moved.ok ? linking(manifestId, wireId, id, links, moved) : moved;
      });
    },
    /** A subtask joins the foot of its parent's. A subtask holds none, so one asked of a subtask is not added. */
    addSubtask: (parentId: string, title: string, links: TaskLinks = {}) => {
      const home = homeOf(parentId);
      if (home === undefined || !rowsIn(home.key).some((row) => row.id === parentId && row.kind === "task")) return;
      const { manifestId, wireId } = home;
      if (home.key !== LOOSE) touch(home.key);
      void queue(manifestId, async () => {
        const before = idsOf(wire.get(manifestId), wireId);
        const added = await source.addProjectSubtask(manifestId, wireId, parentId, { title });
        if (!added.ok) return added;
        const id = made(added.value, wireId, before);
        return id === undefined ? added : linking(manifestId, wireId, id, links, added);
      });
    },
    /** The field that adds a subtask under a task's subtasks, or none. */
    subtasking: (parentId: string | undefined) => set({ subtasking: parentId }),
    /** An issue on a task's card. One the task already carries stays where it is. */
    linkToTask: (rowId: string, issue: Linked) => {
      const home = homeOf(rowId);
      if (home === undefined || home.row.kind !== "task" || (home.row.issues ?? []).some((one) => one.reference === issue.reference)) return;
      void queue(home.manifestId, () => source.linkTaskIssue(home.manifestId, home.wireId, rowId, { issue: issue.reference }));
    },
    /** A Job linked to a task holds it as Dispatch would; one already holding it is let go. */
    linkJob: (rowId: string, job: Holder) => {
      const home = homeOf(rowId);
      if (home !== undefined) void queue(home.manifestId, () => source.linkTaskJob(home.manifestId, home.wireId, rowId, { session_id: job.id }));
    },
    unlinkJob: (rowId: string) => {
      const home = homeOf(rowId);
      if (home === undefined) return;
      editRow(rowId, (row) => {
        if (row.kind !== "task") return row;
        const { job: _gone, ...rest } = row;
        void _gone;
        return rest;
      });
      void queue(home.manifestId, () => source.unlinkTaskJob(home.manifestId, home.wireId, rowId));
    },
    linkPull: (rowId: string, pull: LinkedPull) => {
      const home = homeOf(rowId);
      if (home === undefined || home.row.kind !== "task" || (home.row.pulls ?? []).some((one) => one.number === pull.number)) return;
      void queue(home.manifestId, () => source.linkTaskPull(home.manifestId, home.wireId, rowId, { number: pull.number }));
    },
    unlinkPull: (rowId: string, number: number) => {
      const home = homeOf(rowId);
      if (home === undefined) return;
      editRow(rowId, (row) => (row.kind === "task" ? { ...row, pulls: (row.pulls ?? []).filter((one) => one.number !== number) } : row));
      void queue(home.manifestId, () => source.unlinkTaskPull(home.manifestId, home.wireId, rowId, { number }));
    },
    /** Moves a row one place up or down the list it stands in: a subtask among its parent's. */
    move: (rowId: string, by: 1 | -1) => {
      const home = homeOf(rowId);
      const where = home === undefined ? undefined : siblingsOf(rowsIn(home.key), rowId);
      if (home === undefined || where === undefined) return;
      const at = where.rows.findIndex((row) => row.id === rowId);
      if (at < 0 || at + by < 0 || at + by >= where.rows.length) return;
      edit(home.key, (rows) =>
        withList(rows, where.parent, (list) => {
          [list[at], list[at + by]] = [list[at + by]!, list[at]!];
          return list;
        }),
      );
      remember(home.key);
      const position = placed(rowId);
      if (position !== undefined) void queue(home.manifestId, () => source.moveProjectRow(home.manifestId, home.wireId, rowId, { position }));
    },
    /** Puts a row down before or after another of the same list, as a drag drops it. */
    place: (rowId: string, target: string, after: boolean) => {
      const home = homeOf(rowId);
      const where = home === undefined ? undefined : siblingsOf(rowsIn(home.key), rowId);
      if (home === undefined || where === undefined || rowId === target) return;
      edit(home.key, (rows) =>
        withList(rows, where.parent, (list) => {
          const row = list.find((it) => it.id === rowId);
          const rest = list.filter((it) => it.id !== rowId);
          const at = rest.findIndex((it) => it.id === target);
          if (row === undefined || at < 0) return list;
          rest.splice(after ? at + 1 : at, 0, row);
          return rest;
        }),
      );
      remember(home.key);
      const position = placed(rowId);
      if (position !== undefined) void queue(home.manifestId, () => source.moveProjectRow(home.manifestId, home.wireId, rowId, { position }));
    },
    /** Done, or open again. On a row Fleet asks about, it answers the ask. A task that is done and hidden leaves the selection. */
    tick: (rowId: string) => {
      const home = homeOf(rowId);
      if (home === undefined || home.row.kind !== "task") return;
      const done = !home.row.done;
      editRow(rowId, (one) => {
        if (one.kind !== "task") return one;
        const { asked: _asked, ...rest } = one;
        void _asked;
        return { ...rest, done };
      });
      if (done && !snap.showDone) set({ selected: snap.selected.filter((one) => one !== rowId) });
      void queue(home.manifestId, () => source.tickProjectTask(home.manifestId, home.wireId, rowId, { done }));
    },
    select: (rowId: string) => set({ selected: snap.selected.includes(rowId) ? snap.selected.filter((one) => one !== rowId) : [...snap.selected, rowId], anchor: rowId }),
    /** The cards from the one last clicked to this one join the selection, when both stand in the same list; otherwise this one alone does. Hidden done cards are passed over. */
    selectTo: (rowId: string) => {
      const key = holding(rowId);
      const where = key === undefined ? undefined : siblingsOf(rowsIn(key), rowId);
      const from = where?.rows.findIndex((row) => row.id === snap.anchor) ?? -1;
      const to = where?.rows.findIndex((row) => row.id === rowId) ?? -1;
      if (where === undefined || from < 0 || to < 0) return set({ selected: snap.selected.includes(rowId) ? snap.selected : [...snap.selected, rowId], anchor: rowId });
      const between = where.rows
        .slice(Math.min(from, to), Math.max(from, to) + 1)
        .filter((row) => snap.showDone || !isDone(row))
        .map((row) => row.id);
      set({ selected: [...snap.selected, ...between.filter((one) => !snap.selected.includes(one))], anchor: rowId });
    },
    /** A click that selects nothing still marks where a range will start. */
    anchorAt: (rowId: string) => set({ anchor: rowId }),
    clearSelection: () => set({ selected: [], anchor: undefined }),
    /** A task deleted, or an issue unlinked, with no question: the toast offers it back for a moment. */
    remove: (rowId: string) => {
      const home = homeOf(rowId);
      const where = home === undefined ? undefined : siblingsOf(rowsIn(home.key), rowId);
      if (home === undefined || where === undefined) return;
      const at = where.rows.findIndex((row) => row.id === rowId);
      const slice = sliceOf(where.rows, where.rows[at]!);
      edit(home.key, (rows) => withList(rows, where.parent, (list) => list.filter((row) => row.id !== rowId)));
      set({ selected: snap.selected.filter((one) => one !== rowId), removed: { kind: "row", list: home.key, parent: where.parent, row: where.rows[at]!, at, slice } });
      void queue(home.manifestId, () => source.removeProjectRow(home.manifestId, home.wireId, rowId));
    },
    /** A project deleted from the list, with no question: the toast offers it back, rows and all. */
    removeProject: (projectId: string) => {
      const project = projectOf(projectId);
      if (project === undefined) return;
      const where = homes(projectId);
      set({ projects: snap.projects.filter((one) => one.id !== projectId), removed: { kind: "project", project, homes: where, at: snap.projects.indexOf(project) }, renaming: undefined, ...(snap.open === projectId ? { open: undefined, ...closed } : {}) });
      for (const manifestId of where) void queue(manifestId, () => source.deleteProject(manifestId, projectId));
    },
    /** What he deleted comes back as new rows in Fleet, a project at the foot of the list, since projects keep no place of their own. */
    undo: () => {
      const gone = snap.removed;
      if (gone === undefined) return;
      set({ removed: undefined });
      if (gone.kind === "project") {
        const { name, rows } = gone.project;
        const home = gone.homes[0] ?? rows[0]?.manifestId ?? [...wire.keys()].sort()[0];
        if (home === undefined) return;
        void queue(home, async () => {
          const before = new Set(wire.get(home)?.projects.map((one) => one.id));
          let answer = await source.addProject(home, { name });
          if (!answer.ok) return answer;
          const id = answer.value.projects.find((one) => !before.has(one.id))?.id;
          if (id === undefined) return answer;
          for (const row of rows) {
            if (!answer.ok) return answer;
            answer = await recreate(row.manifestId, id, undefined, row, undefined, row.manifestId === home ? answer.value : wire.get(row.manifestId));
          }
          return answer;
        });
        return;
      }
      if (gone.list !== LOOSE && projectOf(gone.list) === undefined) return;
      edit(gone.list, (rows) => withList(rows, gone.parent, (list) => [...list.slice(0, gone.at), gone.row, ...list.slice(gone.at)]));
      const manifestId = gone.row.manifestId;
      void queue(manifestId, () => recreate(manifestId, gone.list === LOOSE ? ownWire(manifestId) : gone.list, gone.parent, gone.row, gone.slice, wire.get(manifestId)));
    },
    /** A task's title edited in place, or none. */
    editing: (rowId: string | undefined) => set({ editing: rowId }),
    retitle: (rowId: string, title: string) => {
      const home = homeOf(rowId);
      set({ editing: undefined });
      if (home === undefined || home.row.kind !== "task") return;
      const notes = home.row.notes ?? "";
      editRow(rowId, (one) => (one.kind === "task" ? { ...one, title } : one));
      void queue(home.manifestId, () => source.editProjectTask(home.manifestId, home.wireId, rowId, { title, notes }));
    },
    /** A project's name edited in the list, or none. */
    renaming: (projectId: string | undefined) => set({ renaming: projectId }),
    renameProject: (projectId: string, name: string) => {
      set({ renaming: undefined });
      if (projectOf(projectId) === undefined) return;
      set({ projects: snap.projects.map((one) => (one.id === projectId ? { ...one, name } : one)) });
      for (const manifestId of homes(projectId)) void queue(manifestId, () => source.renameProject(manifestId, projectId, { name }));
    },
    forgetRemoved: () => set({ removed: undefined }),
    /** His answer to a done ask: Mark done, or Keep open. Fleet takes the same ask off the Job that raised it. */
    settle: (rowId: string, done: boolean) => {
      const home = homeOf(rowId);
      if (home === undefined || home.row.kind !== "task" || home.row.asked !== true) return;
      editRow(rowId, (one) => {
        if (one.kind !== "task") return one;
        const { asked: _asked, job: _job, ...rest } = one;
        void _asked;
        void _job;
        return { ...rest, done };
      });
      if (done && !snap.showDone) set({ selected: snap.selected.filter((one) => one !== rowId) });
      void queue(home.manifestId, () => source.tickProjectTask(home.manifestId, home.wireId, rowId, { done }));
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

const NEVER: ProjectsSnapshot = { projects: [], loose: [], open: undefined, selected: [], anchor: undefined, subtasking: undefined, removed: undefined, editing: undefined, renaming: undefined, naming: false, lastRepo: undefined, showDone: false, reveal: undefined };
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
