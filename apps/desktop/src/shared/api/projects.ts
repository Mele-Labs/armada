// His projects across repositories, kept in Fleet: every route, and the event that carries them whole. `docs/practices/protocol/projects.md`.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, NameProject, NewProjectIssue, NewProjectTask, Outcome, ProjectTask, Projects, TickProjectTask } from "@armada/protocol";

/** What a read or a write came to: his projects as Fleet holds them now, or the refusal to word. */
export type ProjectsActed = { ok: true; value: Projects } | { ok: false; outcome: Outcome };

export type ProjectsApi = {
  /** His projects, each with its rows in order, and the tasks in none. */
  listProjects: () => Promise<ProjectsActed>;
  addProject: (body: NameProject) => Promise<ProjectsActed>;
  renameProject: (project: string, body: NameProject) => Promise<ProjectsActed>;
  /** Its rows go with it. */
  deleteProject: (project: string) => Promise<ProjectsActed>;
  /** A task, open, in a repository, at the end of the project or, with none, of the loose tasks. */
  addProjectTask: (body: NewProjectTask) => Promise<ProjectsActed>;
  /** A task under the top-level task `row`, at the end of its subtasks. */
  addProjectSubtask: (row: string, body: ProjectTask) => Promise<ProjectsActed>;
  /** A task's title and notes, whole. */
  editProjectTask: (row: string, body: ProjectTask) => Promise<ProjectsActed>;
  /** An issue of a repository joins the project, or the loose rows, as a row. */
  linkProjectIssue: (body: NewProjectIssue) => Promise<ProjectsActed>;
  /** An issue on a task or subtask. Linking it again changes nothing. */
  linkTaskIssue: (row: string, body: LinkProjectIssue) => Promise<ProjectsActed>;
  unlinkTaskIssue: (row: string, body: LinkProjectIssue) => Promise<ProjectsActed>;
  /** A task deleted or an issue row unlinked; a task's subtasks go with it. */
  removeProjectRow: (row: string) => Promise<ProjectsActed>;
  /** To a place in a project or out of every one, keeping its repository. */
  moveProjectRow: (row: string, body: MoveProjectRow) => Promise<ProjectsActed>;
  /** Done, or open again. On a row the Job asked about it answers the ask. */
  tickProjectTask: (row: string, body: TickProjectTask) => Promise<ProjectsActed>;
  /** A live Job of the task's repository holds the task and its subtasks, as if started with them; one holding it is replaced. */
  linkTaskJob: (row: string, body: LinkTaskJob) => Promise<ProjectsActed>;
  /** A held task, and the subtasks the same Job held, go back to open. */
  unlinkTaskJob: (row: string) => Promise<ProjectsActed>;
  /** A pull request of the task's repository, by number, on a task or subtask. */
  linkTaskPull: (row: string, body: LinkTaskPull) => Promise<ProjectsActed>;
  unlinkTaskPull: (row: string, body: LinkTaskPull) => Promise<ProjectsActed>;
  /** `projects.changed`: his projects, whole, after any write to one. */
  onProjectsChanged: (onChanged: (projects: Projects) => void) => () => void;
};

export const PROJECTS_CHANNELS = {
  listProjects: "bridge:list-projects",
  addProject: "bridge:add-project",
  renameProject: "bridge:rename-project",
  deleteProject: "bridge:delete-project",
  addProjectTask: "bridge:add-project-task",
  addProjectSubtask: "bridge:add-project-subtask",
  editProjectTask: "bridge:edit-project-task",
  linkProjectIssue: "bridge:link-project-issue",
  linkTaskIssue: "bridge:link-task-issue",
  unlinkTaskIssue: "bridge:unlink-task-issue",
  removeProjectRow: "bridge:remove-project-row",
  moveProjectRow: "bridge:move-project-row",
  tickProjectTask: "bridge:tick-project-task",
  linkTaskJob: "bridge:link-task-job",
  unlinkTaskJob: "bridge:unlink-task-job",
  linkTaskPull: "bridge:link-task-pull",
  unlinkTaskPull: "bridge:unlink-task-pull",
  /** Main to every window, not a request. */
  projectsChanged: "bridge:projects-changed",
} as const;
