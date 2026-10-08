// What a Trigger is as the editor holds it, and how it goes to and from the file Fleet keeps.
// `get_trigger` answers the file's text and `save_trigger` takes it back, so the draft is read
// from that text and written to it; nothing here reaches Fleet.

import type {
  KeptFrom,
  TriggerDefinition,
  TriggerLevel,
  TriggerMoment,
  TriggerRemoved,
  TriggerSaved,
  TriggerScope,
  TriggerSummary,
} from "@armada/protocol";

import { parse } from "../WorkflowCreator/json";

/** The workflow a draft applies to where it names none: every one. */
export const EVERY = "";

export type TriggerDraft = {
  /** Empty until it is saved, where it takes the name of what it runs. */
  name: string;
  when: TriggerMoment;
  /** Empty is every step, and always on `pr_opened`. */
  step: string;
  /** Empty is every workflow. */
  workflow: string;
  runs: "command" | "skill" | "drone";
  /** The Command, the skill, or the Drone's prompt. */
  with: string;
  block: boolean;
  repair: boolean;
  scope: TriggerScope;
};

/** A Trigger by its identity: when, step and name. The workflow it applies to is not part of it. */
export type TriggerIdentity = { when: TriggerMoment; step?: string; name: string };

export const identityKey = (one: TriggerIdentity) => `${one.when}|${one.step ?? ""}|${one.name}`;

export const blankDraft = (init: Partial<TriggerDraft> = {}): TriggerDraft => ({
  name: "",
  when: "pr_opened",
  step: "",
  workflow: EVERY,
  runs: "command",
  with: "",
  block: false,
  repair: false,
  scope: "repository",
  ...init,
});

type Table = Record<string, unknown>;
const table = (value: unknown): Table =>
  typeof value === "object" && value !== null && !Array.isArray(value) ? (value as Table) : {};
const text = (value: unknown): string | undefined => (typeof value === "string" ? value : undefined);

/** `get_trigger`'s file as a draft. A level that is not the machine's reads as the repository's. */
export function draftOf(read: TriggerDefinition): TriggerDraft {
  const file = table(parse(read.definition));
  const failure = table(file.on_failure);
  const command = text(file.command);
  const brief = text(file.brief);
  return {
    name: text(file.name) ?? read.name,
    when: read.when,
    step: text(file.step) ?? "",
    workflow: text(file.workflow) ?? EVERY,
    runs: command !== undefined ? "command" : brief !== undefined ? "drone" : "skill",
    with: command ?? brief ?? text(file.skill) ?? "",
    block: failure.block === true,
    repair: failure.repair === true,
    scope: read.level === "machine" ? "machine" : "repository",
  };
}

/** What a draft is called: the name it was given, else what it runs, and a Drone's prompt as a slug of its first words. */
export function nameOf(draft: Pick<TriggerDraft, "name" | "runs" | "with">): string {
  if (draft.name !== "") return draft.name;
  if (draft.runs !== "drone") return draft.with;
  return draft.with.toLowerCase().match(/[a-z0-9]+/g)?.slice(0, 4).join("-") ?? "";
}

/** The text `save_trigger` takes. JSON, which the loader's YAML reads the same. */
export function definitionOf(draft: TriggerDraft): string {
  const failure = { ...(draft.block ? { block: true } : {}), ...(draft.repair ? { repair: true } : {}) };
  return JSON.stringify(
    {
      name: nameOf(draft),
      when: draft.when,
      ...(draft.workflow === EVERY ? {} : { workflow: draft.workflow }),
      ...(draft.when === "pr_opened" || draft.step === "" ? {} : { step: draft.step }),
      [draft.runs === "drone" ? "brief" : draft.runs]: draft.with,
      ...(Object.keys(failure).length === 0 ? {} : { on_failure: failure }),
    },
    null,
    2,
  );
}

/** The moment, as a short phrase. */
export function whenSaid(when: TriggerMoment, step?: string): string {
  if (when === "pr_opened") return "PR opened";
  const at = step === undefined || step === "" ? "a step" : step;
  return `${at} ${when === "step_starts" ? "starts" : "passes"}`;
}

/** The Triggers that fire at one step of one workflow, the ones a more specific level replaced left out. */
export function firingAt(
  triggers: readonly TriggerSummary[],
  workflow: string,
  step: { id: string; delivers?: boolean },
): TriggerSummary[] {
  if (step.id === "") return [];
  return triggers.filter(
    (one) =>
      (one.workflow === undefined || one.workflow === workflow) &&
      (one.when === "pr_opened" ? step.delivers === true : one.step === undefined || one.step === step.id),
  );
}

export type TriggerOpened = { ok: true; definition: TriggerDefinition } | { ok: false; said: string };
export type TriggerSavedAnswer = { ok: true; saved: TriggerSaved } | { ok: false; said: string; exists?: true };
export type TriggerRemovedAnswer = { ok: true; removed: TriggerRemoved } | { ok: false; said: string };

/** What the Workflow editor is handed for Triggers: what Fleet lists, and the three things it can be asked to do. */
export type TriggersBinding = {
  triggers: readonly TriggerSummary[];
  /** The Commands the repository's `armada.yml` declares, which a Command Trigger may name. */
  commands: readonly string[];
  onOpen: (identity: TriggerIdentity, level?: TriggerLevel) => Promise<TriggerOpened>;
  /** `keptFrom` is the addition a save keeps for every Job. */
  onSave: (scope: TriggerScope, definition: string, overwrite: boolean, keptFrom?: KeptFrom) => Promise<TriggerSavedAnswer>;
  onRemove: (scope: TriggerScope, identity: TriggerIdentity) => Promise<TriggerRemovedAnswer>;
};
