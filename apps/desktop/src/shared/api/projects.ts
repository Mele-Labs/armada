// A repository's projects, kept in Fleet: every route, and the event that carries them whole. `docs/practices/protocol/projects.md`.
// A slice imports protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { LinkProjectIssue, MoveProjectRow, NameProject, Outcome, ProjectTask, Projects, TickProjectTask } from "@armada/protocol";

/** What a read or a write came to: the repository's projects as Fleet holds them now, or the refusal to word. */
export type ProjectsActed = { ok: true; value: Projects } | { ok: false; outcome: Outcome };

export type ProjectsApi = {
  /** The repository's projects, each with its rows in order. */
  listProjects: (manifestId: string) => Promise<ProjectsActed>;
  addProject: (manifestId: string, body: NameProject) => Promise<ProjectsActed>;
  renameProject: (manifestId: string, project: string, body: NameProject) => Promise<ProjectsActed>;
  /** Its rows go with it. */
  deleteProject: (manifestId: string, project: string) => Promise<ProjectsActed>;
  /** A task, open, at the end of the project's rows. */
  addProjectTask: (manifestId: string, project: string, body: ProjectTask) => Promise<ProjectsActed>;
  /** A task under the top-level task `row`, at the end of its subtasks. */
  addProjectSubtask: (manifestId: string, project: string, row: string, body: ProjectTask) => Promise<ProjectsActed>;
  /** A task's title and notes, whole. */
  editProjectTask: (manifestId: string, project: string, row: string, body: ProjectTask) => Promise<ProjectsActed>;
  /** An issue joins the project as a row. */
  linkProjectIssue: (manifestId: string, project: string, body: LinkProjectIssue) => Promise<ProjectsActed>;
  /** An issue on a task or subtask. Linking it again changes nothing. */
  linkTaskIssue: (manifestId: string, project: string, row: string, body: LinkProjectIssue) => Promise<ProjectsActed>;
  unlinkTaskIssue: (manifestId: string, project: string, row: string, body: LinkProjectIssue) => Promise<ProjectsActed>;
  /** A task deleted or an issue row unlinked; a task's subtasks go with it. */
  removeProjectRow: (manifestId: string, project: string, row: string) => Promise<ProjectsActed>;
  moveProjectRow: (manifestId: string, project: string, row: string, body: MoveProjectRow) => Promise<ProjectsActed>;
  /** Done, or open again. On a row the Job asked about it answers the ask. */
  tickProjectTask: (manifestId: string, project: string, row: string, body: TickProjectTask) => Promise<ProjectsActed>;
  /** `projects.changed`: a repository's projects, whole, after any write to one. */
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
  /** Main to every window, not a request. */
  projectsChanged: "bridge:projects-changed",
} as const;
