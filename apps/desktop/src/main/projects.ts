// His projects, as main asks Fleet for them: one request per route, and the stream's `projects.changed`, passed on to
// every window. Nothing is kept here, since every write answers his projects and the loose tasks whole.

import type { LinkProjectIssue, LinkTaskJob, LinkTaskPull, MoveProjectRow, NameProject, NewProjectIssue, NewProjectTask, Outcome, ProjectTask, Projects, TickProjectTask } from "@armada/protocol";
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

  /** `GET /projects`. */
  list(): Promise<ProjectsActed> {
    return this.act("GET", "");
  }

  /** `POST /projects`. */
  add(body: NameProject): Promise<ProjectsActed> {
    return this.act("POST", "", body);
  }

  /** `POST /projects/project/:project/rename`. */
  rename(project: string, body: NameProject): Promise<ProjectsActed> {
    return this.act("POST", `/project/${seg(project)}/rename`, body);
  }

  /** `POST /projects/project/:project/delete`. */
  delete(project: string): Promise<ProjectsActed> {
    return this.act("POST", `/project/${seg(project)}/delete`);
  }

  /** `POST /projects/tasks`. */
  addTask(body: NewProjectTask): Promise<ProjectsActed> {
    return this.act("POST", "/tasks", body);
  }

  /** `POST /projects/rows/:row/subtasks`. */
  addSubtask(row: string, body: ProjectTask): Promise<ProjectsActed> {
    return this.row(row, "/subtasks", body);
  }

  /** `POST /projects/rows/:row/edit`. */
  editTask(row: string, body: ProjectTask): Promise<ProjectsActed> {
    return this.row(row, "/edit", body);
  }

  /** `POST /projects/issues`. */
  linkIssue(body: NewProjectIssue): Promise<ProjectsActed> {
    return this.act("POST", "/issues", body);
  }

  /** `POST /projects/rows/:row/issues`. */
  linkTaskIssue(row: string, body: LinkProjectIssue): Promise<ProjectsActed> {
    return this.row(row, "/issues", body);
  }

  /** `POST /projects/rows/:row/issues/unlink`. */
  unlinkTaskIssue(row: string, body: LinkProjectIssue): Promise<ProjectsActed> {
    return this.row(row, "/issues/unlink", body);
  }

  /** `POST /projects/rows/:row/job`. */
  linkTaskJob(row: string, body: LinkTaskJob): Promise<ProjectsActed> {
    return this.row(row, "/job", body);
  }

  /** `POST /projects/rows/:row/job/unlink`. */
  unlinkTaskJob(row: string): Promise<ProjectsActed> {
    return this.row(row, "/job/unlink");
  }

  /** `POST /projects/rows/:row/pulls`. */
  linkTaskPull(row: string, body: LinkTaskPull): Promise<ProjectsActed> {
    return this.row(row, "/pulls", body);
  }

  /** `POST /projects/rows/:row/pulls/unlink`. */
  unlinkTaskPull(row: string, body: LinkTaskPull): Promise<ProjectsActed> {
    return this.row(row, "/pulls/unlink", body);
  }

  /** `POST /projects/rows/:row/remove`. */
  removeRow(row: string): Promise<ProjectsActed> {
    return this.row(row, "/remove");
  }

  /** `POST /projects/rows/:row/move`. */
  moveRow(row: string, body: MoveProjectRow): Promise<ProjectsActed> {
    return this.row(row, "/move", body);
  }

  /** `POST /projects/rows/:row/tick`. */
  tickTask(row: string, body: TickProjectTask): Promise<ProjectsActed> {
    return this.row(row, "/tick", body);
  }

  private row(row: string, rest: string, body?: unknown): Promise<ProjectsActed> {
    return this.act("POST", `/rows/${seg(row)}${rest}`, body);
  }

  private async act(method: "GET" | "POST", rest: string, body?: unknown): Promise<ProjectsActed> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, method, `/projects${rest}`, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, value: answer.body as Projects };
  }
}

const seg = encodeURIComponent;
