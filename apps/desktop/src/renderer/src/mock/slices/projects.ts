// The Projects routes, answered from a scenario. A scenario hands each repository's projects as Fleet would list them
// (`MockDraft.projectsTab`), and every write below changes this window's copy and answers it whole, then publishes it as
// `projects.changed`, as Fleet does. A repository's own tasks are a project with the id `ownId(manifest)`, made by the first task
// written to it. **Only a reference or a number is kept on a row**: an issue's or a pull request's title
// and state are read when the rows are, from what the scenario's trackers and pull requests say.
//
// **A done ask is one question in two places**: on the rows, and while the Job that held them is live, as a `task_done` Needs you
// item on that Job. Fleet keeps the two alike; here the window's two stores are joined, as Fleet would join them: the scenario
// hands its Sessions store over (`projectsLink.jobs`) and answers an item through `projectsLink.settle`.

import type { ProjectRow, ProjectRowRef, ProjectTaskState, Projects, WaitingItem } from "@armada/protocol";

import type { ProjectsApi } from "../../../../shared/api/projects";
import { ownId, type LinkedPull } from "../../projects-tab/model";
import type { Slice } from "../fake-context";
import type { SessionsStore } from "../sessions/script";

export type ProjectsSeed = readonly Projects[];

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
    const seed = scenario.draft?.projectsTab ?? [];
    const held = new Map<string, Projects>(structuredClone(seed as Projects[]).map((one) => [one.manifest_id, one]));
    const listeners = new Set<(projects: Projects) => void>();
    let made = 0;
    const fresh = (kind: string) => `${kind}-${Date.now().toString(36)}-${(made += 1)}`;

    // What the trackers and the forge say of a reference and a number: the scenario's, and whatever its seed already read.
    const issueRead = new Map<string, { title?: string; state?: "open" | "closed" }>();
    const pullRead = new Map<number, { title: string; state: "open" | "merged" | "closed" }>();
    for (const one of scenario.draft?.issuesTab ?? []) for (const ticket of one.tickets ?? []) issueRead.set(`${one.tracker.manifestId}:${normal(ticket.reference)}`, { title: ticket.title, state: ticket.state });
    for (const pulls of Object.values(scenario.draft?.projectsPulls ?? {})) for (const pull of pulls as readonly LinkedPull[]) pullRead.set(pull.number, { title: pull.title, state: pull.state === "merged" ? "merged" : pull.state === "closed" ? "closed" : "open" });
    for (const one of held.values()) {
      for (const project of one.projects) {
        for (const row of project.rows) {
          if (row.kind === "issue") issueRead.set(`${one.manifest_id}:${row.issue}`, { ...(row.title === undefined ? {} : { title: row.title }), ...(row.state === undefined ? {} : { state: row.state }) });
          else {
            for (const linked of row.issues ?? []) issueRead.set(`${one.manifest_id}:${linked.issue}`, { ...(linked.title === undefined ? {} : { title: linked.title }), ...(linked.state === undefined ? {} : { state: linked.state }) });
            for (const pull of row.pulls ?? []) if (pull.title !== undefined) pullRead.set(pull.number, { title: pull.title, state: pull.state ?? "open" });
          }
        }
      }
    }

    /** The repository's projects as they are listed: references and numbers read. */
    const listed = (manifestId: string): Projects => {
      const kept = structuredClone(held.get(manifestId) ?? { manifest_id: manifestId, projects: [] });
      for (const project of kept.projects) {
        project.rows = project.rows.map((row): ProjectRow => {
          if (row.kind === "issue") return { kind: "issue", id: row.id, issue: row.issue, ...issueRead.get(`${manifestId}:${row.issue}`) };
          const { issues, pulls, ...rest } = row;
          return {
            ...rest,
            ...((issues ?? []).length === 0 ? {} : { issues: (issues ?? []).map((one) => ({ issue: one.issue, ...issueRead.get(`${manifestId}:${one.issue}`) })) }),
            ...((pulls ?? []).length === 0 ? {} : { pulls: (pulls ?? []).map((one) => ({ number: one.number, ...pullRead.get(one.number) })) }),
          };
        });
      }
      return kept;
    };

    /** Rows are flat, a parent followed by its subtasks: grouped for a change, flat again after. */
    type Group = { row: ProjectRow; subs: Task[] };
    const grouped = (rows: readonly ProjectRow[]): Group[] => {
      const top: Group[] = rows.filter((row) => row.kind === "issue" || row.parent === undefined).map((row) => ({ row, subs: [] }));
      for (const row of rows) if (row.kind === "task" && row.parent !== undefined) top.find((one) => one.row.id === row.parent)?.subs.push(row);
      return top;
    };
    const flat = (groups: readonly Group[]): ProjectRow[] => groups.flatMap((one) => [one.row, ...one.subs]);

    /** Fleet's done ask on each live Job: one `task_done` item per project that holds tasks it asked about, gone with the last. */
    const asked = (): void => {
      const jobs = projectsLink.jobs;
      if (jobs === undefined) return;
      const wanted = new Map<string, Map<string, WaitingItem>>();
      for (const one of held.values()) {
        for (const project of one.projects) {
          const tasks = project.rows.filter((row): row is Task => row.kind === "task" && row.state === "asked" && row.held_by !== undefined);
          for (const job of new Set(tasks.map((row) => row.held_by!.session_id))) {
            const mine = tasks.filter((row) => row.held_by!.session_id === job);
            const item: WaitingItem = {
              id: doneAskId(project.id),
              text: mine.length === 1 ? `Mark ${mine[0]!.title} done?` : `Mark the ${project.name} tasks done?`,
              since: new Date().toISOString(),
              source: "task_done",
              options: [{ label: "Mark done" }, { label: "Keep open" }],
              act: { kind: "answer", target: doneAskId(project.id) },
            };
            wanted.set(job, new Map([...(wanted.get(job) ?? []), [item.id, item]]));
          }
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
    const write = async (manifestId: string, change: (kept: Projects) => ReturnType<typeof refused> | void) => {
      const kept = held.get(manifestId) ?? { manifest_id: manifestId, projects: [] };
      held.set(manifestId, kept);
      const no = change(kept);
      if (no !== undefined) return no;
      asked();
      const answer = listed(manifestId);
      setTimeout(() => listeners.forEach((on) => on(listed(manifestId))), 0);
      return { ok: true, value: answer } as const;
    };
    const inProject = (kept: Projects, id: string) => kept.projects.find((one) => one.id === id);
    const noProject = () => refused("fleet.no_such_project", "There is no such project");
    const noRow = () => refused("fleet.no_such_project_row", "There is no such row");
    const blank = (what: string) => refused("fleet.project_blank", `${what} needs a title`);
    const taskIn = (project: { rows: ProjectRow[] }, id: string): Task | undefined => project.rows.find((row): row is Task => row.kind === "task" && row.id === id);

    projectsLink.settle = (job, projectId, done) => {
      for (const kept of held.values()) {
        const project = kept.projects.find((one) => one.id === projectId);
        if (project === undefined) continue;
        void write(kept.manifest_id, () => {
          for (const row of project.rows) {
            if (row.kind !== "task" || row.state !== "asked" || row.held_by?.session_id !== job) continue;
            row.state = done ? "done" : "open";
            delete row.held_by;
          }
        });
      }
    };

    projectsLink.hold = (job, title, refs) => {
      for (const kept of held.values()) {
        const mine = refs.filter((ref) => kept.projects.some((one) => one.id === ref.project));
        if (mine.length === 0) continue;
        void write(kept.manifest_id, () => {
          for (const ref of mine) {
            const project = kept.projects.find((one) => one.id === ref.project)!;
            for (const row of project.rows) {
              if (row.kind !== "task" || (row.id !== ref.row && row.parent !== ref.row)) continue;
              row.state = "held";
              row.held_by = { session_id: job, title };
            }
          }
        });
      }
    };

    return {
      listProjects: async (manifestId) => ({ ok: true, value: listed(manifestId) }),
      addProject: (manifestId, { name }) =>
        write(manifestId, (kept) => {
          if (name.trim() === "") return blank("A project");
          kept.projects.push({ id: fresh("project"), name: name.trim(), rows: [] });
        }),
      renameProject: (manifestId, project, { name }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          if (name.trim() === "") return blank("A project");
          one.name = name.trim();
        }),
      deleteProject: (manifestId, project) =>
        write(manifestId, (kept) => {
          if (inProject(kept, project) === undefined) return noProject();
          kept.projects = kept.projects.filter((one) => one.id !== project);
        }),
      addProjectTask: (manifestId, project, { title, notes }) =>
        write(manifestId, (kept) => {
          // A repository's own tasks are made by the first one written to them.
          if (project === ownId(manifestId) && inProject(kept, project) === undefined) kept.projects.push({ id: project, name: "", rows: [] });
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          if (title.trim() === "") return blank("A task");
          one.rows.push({ kind: "task", id: fresh("task"), title: title.trim(), notes: (notes ?? "").trim(), state: "open" });
        }),
      addProjectSubtask: (manifestId, project, row, { title, notes }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const parent = taskIn(one, row);
          if (parent === undefined) return noRow();
          if (parent.parent !== undefined) return refused("fleet.project_subtask_too_deep", "A subtask has no subtasks");
          if (title.trim() === "") return blank("A task");
          const groups = grouped(one.rows);
          groups.find((it) => it.row.id === row)?.subs.push({ kind: "task", id: fresh("task"), title: title.trim(), notes: (notes ?? "").trim(), state: "open", parent: row });
          one.rows = flat(groups);
        }),
      editProjectTask: (manifestId, project, row, { title, notes }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          if (title.trim() === "") return blank("A task");
          task.title = title.trim();
          task.notes = (notes ?? "").trim();
        }),
      linkProjectIssue: (manifestId, project, { issue }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const reference = normal(issue);
          if (!one.rows.some((row) => row.kind === "issue" && row.issue === reference)) one.rows.push({ kind: "issue", id: fresh("issue"), issue: reference });
        }),
      linkTaskIssue: (manifestId, project, row, { issue }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          const reference = normal(issue);
          if (!(task.issues ?? []).some((it) => it.issue === reference)) task.issues = [...(task.issues ?? []), { issue: reference }];
        }),
      unlinkTaskIssue: (manifestId, project, row, { issue }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          task.issues = (task.issues ?? []).filter((it) => it.issue !== normal(issue));
          if (task.issues.length === 0) delete task.issues;
        }),
      linkTaskJob: (manifestId, project, row, { session_id }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          const job = projectsLink.jobs?.get().find((it) => it.id === session_id);
          if (job === undefined) return refused("fleet.no_such_session", "There is no such Job");
          for (const each of one.rows) {
            if (each.kind !== "task" || (each.id !== row && each.parent !== row) || each.state === "done") continue;
            each.state = "held";
            each.held_by = { session_id, title: job.title ?? "Job" };
          }
        }),
      unlinkTaskJob: (manifestId, project, row) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          const job = task.held_by?.session_id;
          for (const each of one.rows) {
            if (each.kind !== "task" || (each.id !== row && each.parent !== row) || each.state !== "held" || each.held_by?.session_id !== job) continue;
            each.state = "open";
            delete each.held_by;
          }
        }),
      linkTaskPull: (manifestId, project, row, { number }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          if (!(task.pulls ?? []).some((it) => it.number === number)) task.pulls = [...(task.pulls ?? []), { number }];
        }),
      unlinkTaskPull: (manifestId, project, row, { number }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          task.pulls = (task.pulls ?? []).filter((it) => it.number !== number);
          if (task.pulls.length === 0) delete task.pulls;
        }),
      removeProjectRow: (manifestId, project, row) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          if (!one.rows.some((it) => it.id === row)) return noRow();
          one.rows = one.rows.filter((it) => it.id !== row && !(it.kind === "task" && it.parent === row));
        }),
      moveProjectRow: (manifestId, project, row, { position, parent }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const moving = one.rows.find((it) => it.id === row);
          if (moving === undefined) return noRow();
          const groups = grouped(one.rows);
          const at = groups.findIndex((it) => it.row.id === row);
          const own = moving.kind === "task" ? moving.parent : undefined;
          const into = parent === undefined ? own : parent === "" ? undefined : parent;
          if (into !== undefined) {
            if (moving.kind !== "task") return refused("fleet.project_row_not_a_task", "An issue row cannot go under a task");
            if (at >= 0 && groups[at]!.subs.length > 0) return refused("fleet.project_subtask_too_deep", "A task with subtasks cannot go under another");
          }
          // Out of where it stood, then in at its place among the rows of the parent it ends up under.
          let carried: Group | undefined;
          if (at >= 0) [carried] = groups.splice(at, 1);
          else if (own !== undefined) {
            const mine = groups.find((it) => it.row.id === own)!;
            mine.subs = mine.subs.filter((it) => it.id !== row);
          }
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
          one.rows = flat(groups);
        }),
      tickProjectTask: (manifestId, project, row, { done }) =>
        write(manifestId, (kept) => {
          const one = inProject(kept, project);
          if (one === undefined) return noProject();
          const task = taskIn(one, row);
          if (task === undefined) return noRow();
          task.state = (done ? "done" : "open") satisfies ProjectTaskState;
          delete task.held_by;
        }),
      onProjectsChanged: (onChanged) => {
        listeners.add(onChanged);
        return () => void listeners.delete(onChanged);
      },
    };
  },
};
