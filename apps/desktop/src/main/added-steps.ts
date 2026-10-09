// The two acts on steps added to a Job: add one to a Job underway, and take one off before it fires.
//
// `triggers.ts`'s shape, keyed by Job. The route names the Job and not a repository, so nothing here
// goes through the pick. The read is `JobDetail.additions` and the event is `job.addition_changed`.

import type { AddedStep, AddedStepRemoved, JobDiff } from "@armada/protocol";

import type { AddingStep, AddStepAnswer, EditingStep, EditStepAnswer, ReadingRepairDiff, RemovingStep, RemoveStepAnswer, RepairDiffAnswer } from "../shared/added-steps";
import { ask, route } from "./request";

export class AddedStepCommands {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async add({ jobId, body }: AddingStep): Promise<AddStepAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", route(jobId, "add_job_step"), body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, added: answer.body as AddedStep };
  }

  async remove({ jobId, body }: RemovingStep): Promise<RemoveStepAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", route(jobId, "remove_job_step"), body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, removed: answer.body as AddedStepRemoved };
  }

  async edit({ jobId, body }: EditingStep): Promise<EditStepAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const answer = await ask(port, "POST", route(jobId, "edit_job_step"), body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, edited: answer.body as AddedStep };
  }

  async repairDiff({ jobId, of }: ReadingRepairDiff): Promise<RepairDiffAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const query = new URLSearchParams(of.addition === undefined ? { trigger: of.trigger ?? "" } : { addition: of.addition });
    const answer = await ask(port, "GET", `${route(jobId, "repair_diff")}?${query.toString()}`);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, diff: answer.body as JobDiff };
  }
}
