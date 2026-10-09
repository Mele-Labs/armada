// Triggers' four routes: the list, one definition, a save and a removal.
//
// `workflows.ts`'s shape. A save names the Manifest it is checked against, which is not always the
// pick's: a machine Trigger is checked against whichever repository the person is looking at.

import type { AlertList, ChooseTriggerFix, HoldAct, Outcome, TriggerDefinition, TriggerList, TriggerRemoved, TriggerSaved } from "@armada/protocol";

import type {
  AlertsRead,
  ReadingTrigger,
  RemovingTrigger,
  SavingTrigger,
  TriggerDefinitionRead,
  TriggerRemoveAnswer,
  TriggerSaveAnswer,
  TriggersRead,
} from "../shared/triggers";
import type { Picked } from "./picked";
import { ask, NOT_SET_UP, NO_WAIT, route } from "./request";

/** `list_triggers`, `get_trigger`, `save_trigger`, `remove_trigger`, `choose_trigger_fix`, `rerun_trigger` and `skip_trigger`. */
export class TriggerCommands {
  private readonly port: () => number | null;
  private readonly picked: Picked;

  constructor(port: () => number | null, picked: Picked) {
    this.port = port;
    this.picked = picked;
  }

  /** What Fleet runs for the picked repository, and what it left out. */
  async list(): Promise<TriggersRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.manifest("/triggers");
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, ...(answer.body as TriggerList) };
  }

  /** `list_alerts`: bare on All, the pick's Manifest otherwise. */
  async alerts(): Promise<AlertsRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path = this.picked.narrowed("/alerts");
    if (path === null) return { ok: true, blocked: [], waiting: [] };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, ...(answer.body as AlertList) };
  }

  /** One definition as its file holds it. */
  async definition({ identity, level }: ReadingTrigger): Promise<TriggerDefinitionRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const query = [`when=${identity.when}`, `name=${encodeURIComponent(identity.name)}`];
    if (identity.step !== undefined) query.push(`step=${encodeURIComponent(identity.step)}`);
    if (level !== undefined) query.push(`source=${level}`);
    const path = this.picked.manifest(`/triggers/definition?${query.join("&")}`);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "GET", path);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, definition: answer.body as TriggerDefinition };
  }

  /** Write one. Fleet checks it with the loader's own rules and refuses in its own words. */
  async save({ manifestId, body }: SavingTrigger): Promise<TriggerSaveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path =
      manifestId === null ? this.picked.manifest("/triggers/save") : this.picked.manifestNamed("/triggers/save", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, saved: answer.body as TriggerSaved };
  }

  /** Delete the file that holds one. */
  async remove({ manifestId, body }: RemovingTrigger): Promise<TriggerRemoveAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    const path =
      manifestId === null ? this.picked.manifest("/triggers/remove") : this.picked.manifestNamed("/triggers/remove", manifestId);
    if (path === null) return { ok: false, outcome: NOT_SET_UP };
    const answer = await ask(port, "POST", path, body);
    if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
    return { ok: true, removed: answer.body as TriggerRemoved };
  }

  /**
   * Say where a failed Trigger's held fix goes. **`NO_WAIT`**: Fleet merges, pushes and runs the
   * Command again, and gives up on none of it for a client that stopped waiting, so a bound here
   * would be a guess by the side that knows least. **The answer is only that Fleet took it, and
   * nothing is re-read after**: every state the
   * repair passes through is a `job.trigger_changed`, which re-reads the open Job.
   */
  async chooseFix(jobId: string, body: ChooseTriggerFix): Promise<Outcome> {
    const port = this.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", route(jobId, "choose_trigger_fix"), body, NO_WAIT);
    if (answer.ok !== true) return answer.outcome;
    return { ok: true };
  }

  /**
   * Run a held Trigger's Command again. **`NO_WAIT`**: it takes as long as the Command does. A pass
   * releases the hold and a failure leaves it held, and either way the firing's own
   * `job.trigger_changed` re-reads the Job, so only a refusal is worth telling the caller.
   */
  async rerun(jobId: string, body: HoldAct): Promise<Outcome> {
    return this.act(jobId, "rerun_trigger", body, NO_WAIT);
  }

  /** Let a held Trigger go. Fleet records the firing skipped, by the owner. */
  async skip(jobId: string, body: HoldAct): Promise<Outcome> {
    return this.act(jobId, "skip_trigger", body);
  }

  private async act(jobId: string, verb: string, body: HoldAct, wait?: number): Promise<Outcome> {
    const port = this.port();
    if (port === null) return { ok: false, why: "not_connected" };
    const answer = await ask(port, "POST", route(jobId, verb), body, wait);
    if (answer.ok !== true) return answer.outcome;
    return { ok: true };
  }
}
