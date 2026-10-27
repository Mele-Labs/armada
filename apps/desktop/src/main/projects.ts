// A repository's projects, as main asks Fleet for them: one request per route, and the stream's `projects.changed`, passed on to
// every window. Nothing is kept here, since every write answers the repository's projects whole.

import type { LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, NameProject, Outcome, ProjectTask, Projects, TickProjectTask } from "@armada/protocol";
import type { ProjectsActed } from "../shared/api/projects";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class ProjectsHost {
  private readonly port: () => number | null;
  private told: (projects: Projects) => void = () => {};

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** Say where changed projects go: main's, which owns the windows. */
  onChanged(told: (projects: Projects) => void): void {
    this.told = told;
  }

  /** `projects.changed`. */
  changed(projects: Projects): void {
    this.told(projects);
  }

  /** `GET /projects/:repository`. */
  list(manifestId: string): Promise<ProjectsActed> {
    return this.act("GET", manifestId, "");
  }

  /** `POST /projects/:repository`. */
  add(manifestId: string, body: NameProject): Promise<ProjectsActed> {
    return this.act("POST", manifestId, "", body);
  }

  /** `POST /projects/:repository/:project/rename`. */
  rename(manifestId: string, project: string, body: NameProject): Promise<ProjectsActed> {
    return this.act("POST", manifestId, `/${seg(project)}/rename`, body);
  }

  /** `POST /projects/:repository/:project/delete`. */
  delete(manifestId: string, project: string): Promise<ProjectsActed> {
    return this.act("POST", manifestId, `/${seg(project)}/delete`);
  }

  /** `POST /projects/:repository/:project/tasks`. */
  addTask(manifestId: string, project: string, body: ProjectTask): Promise<ProjectsActed> {
    return this.act("POST", manifestId, `/${seg(project)}/tasks`, body);
  }

  /** `POST /projects/:repository/:project/rows/:row/subtasks`. */
  addSubtask(manifestId: string, project: string, row: string, body: ProjectTask): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/subtasks", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/edit`. */
  editTask(manifestId: string, project: string, row: string, body: ProjectTask): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/edit", body);
  }

  /** `POST /projects/:repository/:project/issues`. */
  linkIssue(manifestId: string, project: string, body: LinkProjectIssue): Promise<ProjectsActed> {
    return this.act("POST", manifestId, `/${seg(project)}/issues`, body);
  }

  /** `POST /projects/:repository/:project/rows/:row/issues`. */
  linkTaskIssue(manifestId: string, project: string, row: string, body: LinkProjectIssue): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/issues", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/issues/unlink`. */
  unlinkTaskIssue(manifestId: string, project: string, row: string, body: LinkProjectIssue): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/issues/unlink", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/job`. */
  linkTaskJob(manifestId: string, project: string, row: string, body: LinkTaskJob): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/job", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/job/unlink`. */
  unlinkTaskJob(manifestId: string, project: string, row: string): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/job/unlink");
  }

  /** `POST /projects/:repository/:project/rows/:row/pulls`. */
  linkTaskPull(manifestId: string, project: string, row: string, body: LinkTaskPull): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/pulls", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/pulls/unlink`. */
  unlinkTaskPull(manifestId: string, project: string, row: string, body: LinkTaskPull): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/pulls/unlink", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/remove`. */
  removeRow(manifestId: string, project: string, row: string): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/remove");
  }

  /** `POST /projects/:repository/:project/rows/:row/move`. */
  moveRow(manifestId: string, project: string, row: string, body: MoveProjectRow): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/move", body);
  }

  /** `POST /projects/:repository/:project/rows/:row/tick`. */
  tickTask(manifestId: string, project: string, row: string, body: TickProjectTask): Promise<ProjectsActed> {
    return this.row(manifestId, project, row, "/tick", body);
  }

  private row(manifestId: string, project: string, row: string, rest: string, body?: unknown): Promise<ProjectsActed> {
    return this.act("POST", manifestId, `/${seg(project)}/rows/${seg(row)}${rest}`, body);
  }

  private async act(method: "GET" | "POST", manifestId: string, rest: string, body?: unknown): Promise<ProjectsActed> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, `/projects/${seg(manifestId)}${rest}`, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as Projects };
  }
}

const seg = encodeURIComponent;
