// The Projects routes, answered from a scenario. A scenario hands his projects as Fleet would list them (`MockDraft.projectsTab`),
// and every write below changes this window's copy and answers it whole, then publishes it as `projects.changed`, as Fleet does.
// **A project is his and spans repositories**: each row names its own, and the tasks in no project are `loose`.
// **Only a reference or a number is kept on a row**: an issue's or a pull request's title and state are read when the rows are,
// from what the scenario's trackers and pull requests say. A link keeps the repository it was linked in, so a task moved to
// another repository leaves its links where they were.
//
// **A done ask is one question in two places**: on the rows, and while the Job that held them is live, as a `task_done` Needs you
// item on that Job. Fleet keeps the two alike; here the window's two stores are joined, as Fleet would join them: the scenario
// hands its Sessions store over (`projectsLink.jobs`) and answers an item through `projectsLink.settle`.

import type { ProjectRow, ProjectRowRef, ProjectTaskState, Projects, WaitingItem } from "@armada/protocol";

import type { ProjectsApi } from "../../../../shared/api/projects";
import type { LinkedPull } from "../../projects-tab/model";
import type { Slice } from "../fake-context";
import type { SessionsStore } from "../sessions/script";

export type ProjectsSeed = Projects;

/** The key the done ask uses for the tasks in no project. */
export const LOOSE_ASK = "loose";

/** The Needs you item a finished Job carries for the tasks it asked about in one project. */
export const doneAskId = (projectId: string): string => `task_done:${projectId}`;
export const projectOfAsk = (itemId: string): string | undefined => (itemId.startsWith("task_done:") ? itemId.slice("task_done:".length) : undefined);

/** This window's halves of the done ask. Set again for each window mounted. */
export const projectsLink: {
  jobs?: SessionsStore;
  settle?: (job: string, projectId: string, done: boolean) => void;
  /** A Job started with these rows holds each written task among them, as Fleet's start does. */
  hold?: (job: string, title: string, rows: readonly ProjectRowRef[]) => void;
} = {};

type Task = Extract<ProjectRow, { kind: "task" }>;

const refused = (code: string, message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code, message, run_id: "mock", fields: {}, chain: [] } } as never }) as const;
const normal = (reference: string): string => reference.trim().replace(/^#/, "");

export const projects: Slice<ProjectsApi, Record<string, never>> = {
  name: "projects",
  state: {},
  api: (scenario) => {
    const held: Projects = structuredClone(scenario.draft?.projectsTab ?? { projects: [], loose: [] });
    const listeners = new Set<(projects: Projects) => void>();
    let made = 0;
    const fresh = (kind: string) => `${kind}-${Date.now().toString(36)}-${(made += 1)}`;

    // What the trackers and the forge say of a reference and a number: the scenario's, and whatever its seed already read.
    const issueRead = new Map<string, { title?: string; state?: "open" | "closed" }>();
    const pullRead = new Map<number, { title: string; state: "open" | "merged" | "closed" }>();
    for (const one of scenario.draft?.issuesTab ?? []) for (const ticket of one.tickets ?? []) issueRead.set(`${one.tracker.manifestId}:${normal(ticket.reference)}`, { title: ticket.title, state: ticket.state });
    for (const pulls of Object.values(scenario.draft?.projectsPulls ?? {})) for (const pull of pulls as readonly LinkedPull[]) pullRead.set(pull.number, { title: pull.title, state: pull.state === "merged" ? "merged" : pull.state === "closed" ? "closed" : "open" });
    /** Every list there is: the tasks in no project, then each project's rows. */
    const lists = (): { id: string; name: string; rows: ProjectRow[] }[] => [{ id: LOOSE_ASK, name: "", rows: held.loose }, ...held.projects];
    for (const list of lists()) {
      for (const row of list.rows) {
        // A seed's link that names no repository was linked in its task's, which a move must not carry it away from.
        if (row.kind === "task") {
          for (const linked of row.issues ?? []) linked.manifest_id ??= row.manifest_id;
          for (const pull of row.pulls ?? []) pull.manifest_id ??= row.manifest_id;
        }
        if (row.kind === "issue") issueRead.set(`${row.manifest_id}:${row.issue}`, { ...(row.title === undefined ? {} : { title: row.title }), ...(row.state === undefined ? {} : { state: row.state }) });
        else {
          for (const linked of row.issues ?? []) issueRead.set(`${linked.manifest_id ?? row.manifest_id}:${linked.issue}`, { ...(linked.title === undefined ? {} : { title: linked.title }), ...(linked.state === undefined ? {} : { state: linked.state }) });
          for (const pull of row.pulls ?? []) if (pull.title !== undefined) pullRead.set(pull.number, { title: pull.title, state: pull.state ?? "open" });
        }
      }
    }

    /** His projects as they are listed: references and numbers read. */
    const listed = (): Projects => {
      const read = (row: ProjectRow): ProjectRow => {
        if (row.kind === "issue") return { kind: "issue", id: row.id, manifest_id: row.manifest_id, issue: row.issue, ...issueRead.get(`${row.manifest_id}:${row.issue}`) };
        const { issues, pulls, ...rest } = row;
        return {
          ...rest,
          ...((issues ?? []).length === 0 ? {} : { issues: (issues ?? []).map((one) => ({ issue: one.issue, manifest_id: one.manifest_id ?? row.manifest_id, ...issueRead.get(`${one.manifest_id ?? row.manifest_id}:${one.issue}`) })) }),
          ...((pulls ?? []).length === 0 ? {} : { pulls: (pulls ?? []).map((one) => ({ number: one.number, manifest_id: one.manifest_id ?? row.manifest_id, ...pullRead.get(one.number) })) }),
        };
      };
      const kept = structuredClone(held);
      return { projects: kept.projects.map((project) => ({ ...project, rows: project.rows.map(read) })), loose: kept.loose.map(read) };
    };

    /** Rows are flat, a parent followed by its subtasks: grouped for a change, flat again after. */
    type Group = { row: ProjectRow; subs: Task[] };
    const grouped = (rows: readonly ProjectRow[]): Group[] => {
      const top: Group[] = rows.filter((row) => row.kind === "issue" || row.parent === undefined).map((row) => ({ row, subs: [] }));
      for (const row of rows) if (row.kind === "task" && row.parent !== undefined) top.find((one) => one.row.id === row.parent)?.subs.push(row);
      return top;
    };
    const flat = (groups: readonly Group[]): ProjectRow[] => groups.flatMap((one) => [one.row, ...one.subs]);
    const reset = (rows: ProjectRow[], groups: readonly Group[]): void => void rows.splice(0, rows.length, ...flat(groups));

    /** Fleet's done ask on each live Job: one `task_done` item per project that holds tasks it asked about, gone with the last. */
    const asked = (): void => {
      const jobs = projectsLink.jobs;
      if (jobs === undefined) return;
      const wanted = new Map<string, Map<string, WaitingItem>>();
      for (const list of lists()) {
        const tasks = list.rows.filter((row): row is Task => row.kind === "task" && row.state === "asked" && row.held_by !== undefined);
        for (const job of new Set(tasks.map((row) => row.held_by!.session_id))) {
          const mine = tasks.filter((row) => row.held_by!.session_id === job);
          const item: WaitingItem = {
            id: doneAskId(list.id),
            text: mine.length === 1 ? `Mark ${mine[0]!.title} done?` : list.name === "" ? "Mark the tasks done?" : `Mark the ${list.name} tasks done?`,
            since: new Date().toISOString(),
            source: "task_done",
            options: [{ label: "Mark done" }, { label: "Keep open" }],
            act: { kind: "answer", target: doneAskId(list.id) },
          };
          wanted.set(job, new Map([...(wanted.get(job) ?? []), [item.id, item]]));
        }
      }
      for (const session of jobs.get()) {
        const ours = (session.waitingFor ?? []).filter((it) => it.source === "task_done");
        const now = wanted.get(session.id) ?? new Map<string, WaitingItem>();
        const same = ours.length === now.size && ours.every((it) => now.has(it.id));
        if (same || session.dead !== undefined) continue;
        jobs.edit(session.id, (one) => ({
          ...one,
          waitingFor: [...(one.waitingFor ?? []).filter((it) => it.source !== "task_done"), ...[...now.values()].map((it) => ours.find((was) => was.id === it.id) ?? it)],
        }));
      }
    };

    /** A write: changes this window's copy, answers it whole and, a moment after, publishes it. */
    const write = async (change: () => ReturnType<typeof refused> | void) => {
      const no = change();
      if (no !== undefined) return no;
      asked();
      const answer = listed();
      setTimeout(() => listeners.forEach((on) => on(listed())), 0);
      return { ok: true, value: answer } as const;
    };
    const noProject = () => refused("fleet.no_such_project", "There is no such project");
    const noRow = () => refused("fleet.no_such_project_row", "There is no such row");
    const blank = (what: string) => refused("fleet.project_blank", `${what} needs a title`);
    /** The list a row stands in, and the row. */
    const locate = (id: string): { rows: ProjectRow[]; row: ProjectRow } | undefined => {
      for (const list of lists()) {
        const row = list.rows.find((one) => one.id === id);
        if (row !== undefined) return { rows: list.rows, row };
      }
      return undefined;
    };
    const taskAt = (id: string): { rows: ProjectRow[]; row: Task } | undefined => {
      const at = locate(id);
      return at?.row.kind === "task" ? { rows: at.rows, row: at.row } : undefined;
    };
    /** The rows a project holds, or the loose ones for none. */
    const within = (project: string | undefined): ProjectRow[] | undefined => (project === undefined || project === "" ? held.loose : held.projects.find((one) => one.id === project)?.rows);

    projectsLink.settle = (job, projectId, done) => {
      const list = lists().find((one) => one.id === projectId);
      if (list === undefined) return;
      void write(() => {
        for (const row of list.rows) {
          if (row.kind !== "task" || row.state !== "asked" || row.held_by?.session_id !== job) continue;
          row.state = done ? "done" : "open";
          delete row.held_by;
        }
      });
    };

    projectsLink.hold = (job, title, refs) => {
      void write(() => {
        for (const ref of refs) {
          const at = locate(ref.row);
          if (at === undefined) continue;
          for (const row of at.rows) {
            if (row.kind !== "task" || (row.id !== ref.row && row.parent !== ref.row)) continue;
            row.state = "held";
            row.held_by = { session_id: job, title };
          }
        }
      });
    };

    return {
      listProjects: async () => ({ ok: true, value: listed() }),
      addProject: ({ name }) =>
        write(() => {
          if (name.trim() === "") return blank("A project");
          held.projects.push({ id: fresh("project"), name: name.trim(), rows: [] });
        }),
      renameProject: (project, { name }) =>
        write(() => {
          const one = held.projects.find((it) => it.id === project);
          if (one === undefined) return noProject();
          if (name.trim() === "") return blank("A project");
          one.name = name.trim();
        }),
      deleteProject: (project) =>
        write(() => {
          if (!held.projects.some((it) => it.id === project)) return noProject();
          held.projects = held.projects.filter((one) => one.id !== project);
        }),
      addProjectTask: ({ manifest_id, project, title, notes }) =>
        write(() => {
          const rows = within(project);
          if (rows === undefined) return noProject();
          if (title.trim() === "") return blank("A task");
          rows.push({ kind: "task", id: fresh("task"), manifest_id, title: title.trim(), notes: (notes ?? "").trim(), state: "open" });
        }),
      addProjectSubtask: (row, { title, notes }) =>
        write(() => {
          const parent = taskAt(row);
          if (parent === undefined) return locate(row) === undefined ? noRow() : refused("fleet.project_row_not_a_task", "An issue row has no subtasks");
          if (parent.row.parent !== undefined) return refused("fleet.project_subtask_too_deep", "A subtask has no subtasks");
          if (title.trim() === "") return blank("A task");
          const groups = grouped(parent.rows);
          groups.find((it) => it.row.id === row)?.subs.push({ kind: "task", id: fresh("task"), manifest_id: parent.row.manifest_id, title: title.trim(), notes: (notes ?? "").trim(), state: "open", parent: row });
          reset(parent.rows, groups);
        }),
      editProjectTask: (row, { title, notes }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          if (title.trim() === "") return blank("A task");
          task.row.title = title.trim();
          task.row.notes = (notes ?? "").trim();
        }),
      linkProjectIssue: ({ manifest_id, project, issue }) =>
        write(() => {
          const rows = within(project);
          if (rows === undefined) return noProject();
          const reference = normal(issue);
          if (!rows.some((row) => row.kind === "issue" && row.manifest_id === manifest_id && row.issue === reference)) rows.push({ kind: "issue", id: fresh("issue"), manifest_id, issue: reference });
        }),
      linkTaskIssue: (row, { issue }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          const reference = normal(issue);
          if (!(task.row.issues ?? []).some((it) => it.issue === reference)) task.row.issues = [...(task.row.issues ?? []), { issue: reference, manifest_id: task.row.manifest_id }];
        }),
      unlinkTaskIssue: (row, { issue }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          task.row.issues = (task.row.issues ?? []).filter((it) => it.issue !== normal(issue));
          if (task.row.issues.length === 0) delete task.row.issues;
        }),
      linkTaskJob: (row, { session_id }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          const job = projectsLink.jobs?.get().find((it) => it.id === session_id);
          if (job === undefined) return refused("fleet.no_such_session", "There is no such Job");
          for (const each of task.rows) {
            if (each.kind !== "task" || (each.id !== row && each.parent !== row) || each.state === "done") continue;
            each.state = "held";
            each.held_by = { session_id, title: job.title ?? "Job" };
          }
        }),
      unlinkTaskJob: (row) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          const job = task.row.held_by?.session_id;
          for (const each of task.rows) {
            if (each.kind !== "task" || (each.id !== row && each.parent !== row) || each.state !== "held" || each.held_by?.session_id !== job) continue;
            each.state = "open";
            delete each.held_by;
          }
        }),
      linkTaskPull: (row, { number }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          if (!(task.row.pulls ?? []).some((it) => it.number === number)) task.row.pulls = [...(task.row.pulls ?? []), { number, manifest_id: task.row.manifest_id }];
        }),
      unlinkTaskPull: (row, { number }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          task.row.pulls = (task.row.pulls ?? []).filter((it) => it.number !== number);
          if (task.row.pulls.length === 0) delete task.row.pulls;
        }),
      removeProjectRow: (row) =>
        write(() => {
          const at = locate(row);
          if (at === undefined) return noRow();
          at.rows.splice(0, at.rows.length, ...at.rows.filter((it) => it.id !== row && !(it.kind === "task" && it.parent === row)));
        }),
      moveProjectRow: (row, { project, position, parent }) =>
        write(() => {
          const from = locate(row);
          if (from === undefined) return noRow();
          const to = project === undefined ? from.rows : within(project);
          if (to === undefined) return noProject();
          const moving = from.row;
          const across = to !== from.rows;
          const own = moving.kind === "task" ? moving.parent : undefined;
          const into = parent === undefined ? (across ? undefined : own) : parent === "" ? undefined : parent;
          const leaving = grouped(from.rows);
          const at = leaving.findIndex((it) => it.row.id === row);
          if (into !== undefined) {
            if (moving.kind !== "task") return refused("fleet.project_row_not_a_task", "An issue row cannot go under a task");
            if (at >= 0 && leaving[at]!.subs.length > 0) return refused("fleet.project_subtask_too_deep", "A task with subtasks cannot go under another");
          }
          // Out of where it stood, then in at its place among the rows of the parent it ends up under.
          let carried: Group | undefined;
          if (at >= 0) [carried] = leaving.splice(at, 1);
          else if (own !== undefined) {
            const mine = leaving.find((it) => it.row.id === own)!;
            mine.subs = mine.subs.filter((it) => it.id !== row);
          }
          const groups = across ? grouped(to) : leaving;
          if (into === undefined) {
            const base: Group = carried ?? { row: moving, subs: [] };
            if (base.row.kind === "task") {
              const { parent: _gone, ...top } = base.row;
              void _gone;
              base.row = top;
            }
            groups.splice(Math.min(position, groups.length), 0, base);
          } else {
            const target = groups.find((it) => it.row.id === into);
            if (target === undefined || moving.kind !== "task") return noRow();
            target.subs.splice(Math.min(position, target.subs.length), 0, { ...moving, parent: into });
          }
          reset(from.rows, leaving);
          if (across) reset(to, groups);
        }),
      moveProjectTaskRepository: (row, { manifest_id }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          if (task.row.parent !== undefined) return refused("fleet.project_subtask_too_deep", "A subtask moves with its task");
          // The links are kept as they are: each was made in its own repository and stays there.
          for (const each of task.rows) if (each.kind === "task" && (each.id === row || each.parent === row)) each.manifest_id = manifest_id;
        }),
      tickProjectTask: (row, { done }) =>
        write(() => {
          const task = taskAt(row);
          if (task === undefined) return noRow();
          task.row.state = (done ? "done" : "open") satisfies ProjectTaskState;
          delete task.row.held_by;
        }),
      onProjectsChanged: (onChanged) => {
        listeners.add(onChanged);
        return () => void listeners.delete(onChanged);
      },
    };
  },
};
