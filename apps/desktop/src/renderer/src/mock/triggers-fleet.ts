// The Triggers' four routes on the mock Fleet: the list, one definition, a save and a removal,
// answered from files held in memory. **Every answer is the type `@armada/protocol` names for it**,
// built the way Fleet builds it (`fleet::trigger_authoring`): the winner of each identity with the
// copies it replaced, a refusal in Fleet's own code, a repository save that waits for `main`, and
// a machine save that is skipped where the repository lacks the Command.

import type {
  Outcome,
  OverriddenTrigger,
  TriggerDefinition,
  TriggerLevel,
  TriggerMoment,
  TriggerRuns,
  TriggerSaved,
  TriggerScope,
  TriggerSkip,
  TriggerSummary,
} from "@armada/protocol";
import { DECLARED } from "@armada/manifest/fake";

import type { BridgeApi } from "../../../shared/api";

type Served = Pick<BridgeApi, "readTriggers" | "readTrigger" | "saveTrigger" | "removeTrigger">;

const refusal = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: { code, message, run_id: "mock", fields: {}, chain: [] },
});

/** The Commands this repository declares: the Manifest the mock serves. */
const DECLARED_COMMANDS = DECLARED.commands.map((one) => one.name);

const FOLDER: Record<TriggerScope, string> = {
  repository: ".armada/triggers",
  machine: "~/.armada/machine/triggers",
};

/** What a file says, as the loader reads it. */
type Held = {
  scope: TriggerScope;
  file: string;
  name: string;
  when: TriggerMoment;
  step?: string;
  workflow?: string;
  runs: TriggerRuns;
  block: boolean;
  repair: boolean;
  text: string;
};

const RANK: Record<TriggerLevel, number> = { armada: 0, repository: 1, machine: 2 };

const identity = (one: { when: TriggerMoment; step?: string | undefined; name: string }) => `${one.when}|${one.step ?? ""}|${one.name}`;

function held(scope: TriggerScope, file: Omit<Held, "scope" | "file" | "text" | "block" | "repair"> & { block?: boolean; repair?: boolean }): Held {
  const { block = false, repair = false, ...rest } = file;
  const text = JSON.stringify(
    {
      name: rest.name,
      when: rest.when,
      ...(rest.workflow === undefined ? {} : { workflow: rest.workflow }),
      ...(rest.step === undefined ? {} : { step: rest.step }),
      [rest.runs.kind]: rest.runs.name,
      ...(block || repair ? { on_failure: { ...(block ? { block } : {}), ...(repair ? { repair } : {}) } } : {}),
    },
    null,
    2,
  );
  return { scope, file: `${FOLDER[scope]}/${rest.name}.yml`, block, repair, text, ...rest };
}

/** What a fresh Fleet holds: the repository's, and the machine's over one of them. */
const SEEDED: Held[] = [
  held("repository", { name: "fmt", when: "step_passes", step: "implement", runs: { kind: "command", name: "fmt" } }),
  held("repository", { name: "gate", when: "pr_opened", runs: { kind: "command", name: "gate" }, block: true }),
  held("machine", { name: "gate", when: "pr_opened", runs: { kind: "command", name: "gate" }, repair: true }),
  held("machine", { name: "deploy_qa", when: "pr_opened", runs: { kind: "command", name: "deploy_qa" } }),
  held("repository", { name: "qa-notes", when: "step_starts", step: "tests", workflow: "feature", runs: { kind: "skill", name: "qa-notes" } }),
];

/** Why a Trigger does not run in this repository, where it does not. */
function skippedOf(runs: TriggerRuns): TriggerSkip | undefined {
  if (runs.kind === "skill") return { reason: "skill_not_run", name: runs.name, said: "A skill is not run yet" };
  return DECLARED_COMMANDS.includes(runs.name)
    ? undefined
    : { reason: "not_in_this_repo", name: runs.name, said: `\`${runs.name}\` is not a Command this repository declares` };
}

/** `GET /triggers`: the copy that runs for each identity, and the lower ones it replaced. */
function listed(files: readonly Held[]): TriggerSummary[] {
  const byIdentity = new Map<string, Held[]>();
  for (const one of files) byIdentity.set(identity(one), [...(byIdentity.get(identity(one)) ?? []), one]);
  return [...byIdentity.values()].map((copies) => {
    const [winner, ...under] = [...copies].sort((a, b) => RANK[b.scope] - RANK[a.scope]) as [Held, ...Held[]];
    const skipped = skippedOf(winner.runs);
    const overrides: OverriddenTrigger[] = under.map((one) => ({ level: one.scope, file: one.file }));
    return {
      name: winner.name,
      when: winner.when,
      ...(winner.workflow === undefined ? {} : { workflow: winner.workflow }),
      ...(winner.step === undefined ? {} : { step: winner.step }),
      runs: winner.runs,
      block: winner.block,
      repair: winner.repair,
      level: winner.scope,
      file: winner.file,
      ...(skipped === undefined ? {} : { skipped }),
      ...(overrides.length === 0 ? {} : { overrides }),
    };
  });
}

/** A fresh Fleet's Triggers. */
export function triggersServed(): Served {
  let files: Held[] = [...SEEDED];
  const at = (scope: TriggerScope, key: string) => files.find((one) => one.scope === scope && identity(one) === key);
  const winnerOf = (key: string): TriggerLevel | undefined => files.filter((one) => identity(one) === key).sort((a, b) => RANK[b.scope] - RANK[a.scope])[0]?.scope;
  const none = (name: string) => refusal("fleet.no_such_trigger", `No Trigger \`${name}\` is held there`);
  return {
    readTriggers: async () => ({ ok: true, triggers: listed(files), left_out: [] }),
    readTrigger: async ({ identity: asked, level }) => {
      const key = identity(asked);
      const copies = files.filter((one) => identity(one) === key).sort((a, b) => RANK[b.scope] - RANK[a.scope]);
      const one = level === undefined ? copies[0] : copies.find((c) => c.scope === level);
      if (one === undefined) return { ok: false, outcome: none(asked.name) };
      const definition: TriggerDefinition = {
        name: one.name,
        when: one.when,
        ...(one.step === undefined ? {} : { step: one.step }),
        level: one.scope,
        file: one.file,
        definition: one.text,
        ...(copies[0] !== undefined && copies[0] !== one ? { overridden_by: copies[0].scope } : {}),
      };
      return { ok: true, definition };
    },
    saveTrigger: async ({ body }) => {
      const parsed = JSON.parse(body.definition) as Record<string, unknown>;
      const name = String(parsed.name ?? "");
      if (!/^[A-Za-z0-9_-]+$/.test(name)) {
        return { ok: false, outcome: refusal("fleet.trigger_name_not_a_name", `\`${name}\` cannot be part of a file's name: letters, digits, \`-\` and \`_\``) };
      }
      const step = typeof parsed.step === "string" ? parsed.step : undefined;
      const runs: TriggerRuns =
        typeof parsed.command === "string" ? { kind: "command", name: parsed.command } : { kind: "skill", name: String(parsed.skill) };
      const failure = (parsed.on_failure ?? {}) as { block?: boolean; repair?: boolean };
      const next = held(body.scope, {
        name,
        when: parsed.when as TriggerMoment,
        ...(step === undefined ? {} : { step }),
        ...(typeof parsed.workflow === "string" ? { workflow: parsed.workflow } : {}),
        runs,
        block: failure.block === true,
        repair: failure.repair === true,
      });
      const skipped = skippedOf(runs);
      // A repository's save is refused for a Command its `armada.yml` lacks; a machine's is not.
      if (body.scope === "repository" && skipped?.reason === "not_in_this_repo") {
        return { ok: false, outcome: refusal("fleet.trigger_unfit", `\`command\`: ${skipped.said}`) };
      }
      const before = at(body.scope, identity(next));
      if (before !== undefined && body.overwrite !== true) {
        return { ok: false, outcome: refusal("fleet.trigger_exists", `${before.file} already defines \`${name}\` and nothing was written; save again with overwrite to replace it`) };
      }
      files = [...files.filter((one) => one !== before), next];
      const saved: TriggerSaved = {
        name,
        when: next.when,
        ...(step === undefined ? {} : { step }),
        scope: body.scope,
        file: next.file,
        replaced: before !== undefined,
        ...(winnerOf(identity(next)) === undefined ? {} : { runs_from: winnerOf(identity(next))! }),
        ...(body.scope === "repository" ? { waits_for_main: true } : {}),
        ...(body.scope === "machine" && skipped !== undefined ? { skipped } : {}),
      };
      return { ok: true, saved };
    },
    removeTrigger: async ({ body }) => {
      const key = identity(body);
      const one = at(body.scope, key);
      if (one === undefined) return { ok: false, outcome: none(body.name) };
      files = files.filter((other) => other !== one);
      const runs_from = winnerOf(key);
      return { ok: true, removed: { scope: body.scope, file: one.file, ...(runs_from === undefined ? {} : { runs_from }), ...(body.scope === "repository" ? { waits_for_main: true } : {}) } };
    },
  };
}
