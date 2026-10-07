import { useEffect, useState } from "react";
import { Briefcase, Check, CircleDashed, CircleDot, Eye, FolderGit2, GitBranch, Minus, Package, SquarePlus, Trash2, Webhook, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type {
  AddedRuns,
  AddedStep,
  JobTrigger,
  KeptFrom,
  TriggerFiringState,
  TriggerLevel,
  TriggerSaved,
  TriggerScope,
  TriggerSkip,
  TriggerSummary,
} from "@armada/protocol";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";
import {
  blankDraft,
  definitionOf,
  draftOf,
  EVERY,
  identityKey,
  whenSaid,
  type TriggerDraft,
  type TriggerIdentity,
  type TriggersBinding,
} from "./triggers";

/**
 * Triggers: a Command or a skill that fires at a moment in a Job. The rows are the list the
 * Workflow editor draws twice, beside the workflows and on a step, and each opens the same
 * fields; `FiredTriggers` is what Job detail draws once they have fired. Fleet keeps them, and
 * the editor's `TriggersBinding` is the only way here to it.
 */

/** What each level is, said once on its mark. */
const LEVEL: Record<TriggerLevel, { Glyph: LucideIcon; said: string }> = {
  armada: { Glyph: Package, said: "Ships with Armada" },
  repository: { Glyph: FolderGit2, said: "This repository" },
  machine: { Glyph: Briefcase, said: "This machine" },
};

/** A small mark with its tooltip. `pulsing` is the one thing at work. */
function Mark({ icon: Glyph, label, hue, pulsing = false }: { icon: LucideIcon; label: string; hue?: string; pulsing?: boolean }) {
  return (
    <Tooltip label={label}>
      <span className="armada-triggers__mark" {...(hue === undefined ? {} : { "data-hue": hue })} {...(pulsing ? { "data-pulsing": "" } : {})} role="img" aria-label={label}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

export function LevelMark({ level }: { level: TriggerLevel }) {
  const { Glyph, said } = LEVEL[level];
  return <Mark icon={Glyph} label={said} />;
}

/** A Trigger that does not run here, and why in Fleet's own words. */
function SkippedMark({ skipped }: { skipped: TriggerSkip }) {
  return <Mark icon={Minus} label={skipped.said} />;
}

/** A repository's Trigger is read from `main`, so one saved there runs once it lands. */
function WaitsMark() {
  return <Mark icon={GitBranch} label="Runs once it is on main" />;
}

export type TriggerOpen = (summary: TriggerSummary, level?: TriggerLevel) => void;

/**
 * The Triggers as rows, each marked by the level that sets it. **A copy a more specific level
 * replaced is drawn beneath the one that runs, struck through**, and opens as it is written.
 * `saved` holds what a save answered for a row, which is where `waits_for_main` is shown.
 */
export function TriggerRows({
  triggers,
  label,
  saved,
  onOpen,
}: {
  triggers: readonly TriggerSummary[];
  label: string;
  saved?: ReadonlyMap<string, TriggerSaved>;
  onOpen: TriggerOpen;
}) {
  return (
    <ul className="armada-triggers" aria-label={label}>
      {triggers.map((one) => {
        const answered = saved?.get(identityKey(one));
        return (
          <li key={`${one.level}|${identityKey(one)}|${one.workflow ?? ""}`}>
            <div className="armada-triggers__row">
              <button type="button" aria-label={`${one.name}, ${whenSaid(one.when, one.step)}, ${LEVEL[one.level].said.toLowerCase()}`} onClick={() => onOpen(one)}>
                <LevelMark level={one.level} />
                <span className="armada-triggers__name">{one.name}</span>
                {one.skipped === undefined ? null : <SkippedMark skipped={one.skipped} />}
                {answered?.waits_for_main === true ? <WaitsMark /> : null}
                <span className="armada-triggers__when">{whenSaid(one.when, one.step)}</span>
              </button>
            </div>
            {(one.overrides ?? []).map((under) => (
              <div key={`${under.level}|${under.file}`} className="armada-triggers__row" data-state="overridden">
                <button type="button" aria-label={`${one.name}, ${whenSaid(one.when, one.step)}, ${LEVEL[under.level].said.toLowerCase()}, replaced`} onClick={() => onOpen(one, under.level)}>
                  <LevelMark level={under.level} />
                  <span className="armada-triggers__name">{one.name}</span>
                  <span className="armada-triggers__when">{whenSaid(one.when, one.step)}</span>
                </button>
              </div>
            ))}
          </li>
        );
      })}
    </ul>
  );
}

/** The two switches a Trigger carries. */
function Fails({
  block,
  repair,
  onChange,
  disabled = false,
}: {
  block: boolean;
  repair: boolean;
  onChange: (next: { block?: boolean; repair?: boolean }) => void;
  /** An added step already on a Job is read: Fleet has no edit for one, so the switches are set before it is added. */
  disabled?: boolean;
}) {
  return (
    <div className="armada-triggers__fails" role="group" aria-label="If it fails">
      <span className="armada-triggers__eyebrow">If it fails</span>
      <Switch checked={block} disabled={disabled} onChange={(event) => onChange({ block: event.target.checked })}>
        Block the Job
      </Switch>
      <Switch checked={repair} disabled={disabled} onChange={(event) => onChange({ repair: event.target.checked })}>
        Self repair
      </Switch>
    </div>
  );
}

/** The fields of one Trigger. */
function TriggerFields({
  draft,
  workflow,
  steps,
  commands,
  onChange,
}: {
  draft: TriggerDraft;
  /** The workflow open beside the panel, which "This workflow" means. Absent where none is. */
  workflow: string | undefined;
  /** That workflow's steps. */
  steps: readonly string[];
  commands: readonly string[];
  onChange: (next: Partial<TriggerDraft>) => void;
}) {
  const held = draft.with === "" || commands.includes(draft.with) ? commands : [...commands, draft.with];
  const here = draft.workflow !== EVERY && draft.workflow === workflow;
  return (
    <>
      <Select label="When" value={draft.when} onChange={(event) => onChange({ when: event.target.value as TriggerDraft["when"] })}>
        <option value="step_starts">A step starts</option>
        <option value="step_passes">A step passes</option>
        <option value="pr_opened">PR opened</option>
      </Select>
      {draft.when === "pr_opened" ? null : here ? (
        <Select label="Step" value={draft.step} onChange={(event) => onChange({ step: event.target.value })}>
          <option value="" />
          {steps.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
      ) : (
        <Input label="Step" mono value={draft.step} onChange={(event) => onChange({ step: event.target.value })} />
      )}
      <Select label="Applies to" value={draft.workflow} onChange={(event) => onChange({ workflow: event.target.value })}>
        <option value={EVERY}>Every workflow</option>
        {workflow === undefined ? null : <option value={workflow}>{`This workflow, ${workflow}`}</option>}
        {draft.workflow === EVERY || draft.workflow === workflow ? null : <option value={draft.workflow}>{`Workflow ${draft.workflow}`}</option>}
      </Select>
      <Select label="Set in" value={draft.scope} onChange={(event) => onChange({ scope: event.target.value as TriggerScope })}>
        <option value="repository">Repository</option>
        <option value="machine">This machine</option>
      </Select>
      <Select label="Runs" value={draft.runs} onChange={(event) => onChange({ runs: event.target.value as TriggerDraft["runs"], with: "" })}>
        <option value="command">Command</option>
        <option value="skill">Skill</option>
      </Select>
      {draft.runs === "command" ? (
        <Select label="Command" value={draft.with} onChange={(event) => onChange({ with: event.target.value })}>
          <option value="" />
          {held.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
      ) : (
        <Input label="Skill" mono value={draft.with} onChange={(event) => onChange({ with: event.target.value })} />
      )}
      <Fails block={draft.block} repair={draft.repair} onChange={onChange} />
    </>
  );
}

/** What is open in the panel: a new Trigger, or one of those the list holds at a level. */
export type TriggerTarget =
  | { kind: "new"; init: Partial<TriggerDraft> }
  | { kind: "open"; summary: TriggerSummary; level?: TriggerLevel };

type Standing = "draft" | "refused" | "saved";

/**
 * One Trigger in the editor's overlay panel: read, edited and saved through the binding.
 * **Fleet's sentence is drawn as it said it**, and a second Save replaces where it said one is
 * there. A Trigger moved to another level, moment or step is saved where it now is and removed
 * from where it was, so the old file does not run beside the new.
 */
export function TriggerSheet({
  binding,
  target,
  workflow,
  steps,
  onSaved,
  onRemoved,
  onClose,
  keeping,
}: {
  binding: TriggersBinding;
  target: TriggerTarget;
  /** The addition this save keeps for every Job: Fleet records where it went. */
  keeping?: KeptFrom;
  workflow: string | undefined;
  steps: readonly string[];
  onSaved: (saved: TriggerSaved) => void;
  onRemoved: () => void;
  onClose: () => void;
}) {
  const [was, setWas] = useState<TriggerDraft | null>(target.kind === "new" ? blankDraft(target.init) : null);
  const [draft, setDraft] = useState<TriggerDraft | null>(was);
  const [standing, setStanding] = useState<Standing>("draft");
  const [said, setSaid] = useState("");
  const [replaces, setReplaces] = useState(false);
  const [answer, setAnswer] = useState<TriggerSaved | null>(null);

  useEffect(() => {
    if (target.kind !== "open") return;
    let alive = true;
    void binding.onOpen(target.summary, target.level).then((read) => {
      if (!alive) return;
      if (read.ok) {
        const opened = draftOf(read.definition);
        setWas(opened);
        setDraft(opened);
      } else {
        setStanding("refused");
        setSaid(read.said);
      }
    });
    return () => {
      alive = false;
    };
    // The binding is the surface's and changes with every list it reads.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [target]);

  const change = (next: Partial<TriggerDraft>) => {
    setStanding("draft");
    setReplaces(false);
    setDraft((now) => (now === null ? now : { ...now, ...next, ...(next.when === "pr_opened" ? { step: "" } : {}) }));
  };

  async function save() {
    if (draft === null) return;
    const identity: TriggerIdentity = { when: draft.when, ...(draft.step === "" || draft.when === "pr_opened" ? {} : { step: draft.step }), name: draft.name === "" ? draft.with : draft.name };
    const result = await binding.onSave(draft.scope, definitionOf(draft), replaces || (was !== null && target.kind === "open" && !moved(was, draft)), keeping);
    if (!result.ok) {
      setStanding("refused");
      setSaid(result.said);
      setReplaces(result.exists === true);
      return;
    }
    if (was !== null && target.kind === "open" && moved(was, draft)) {
      await binding.onRemove(was.scope, { when: was.when, ...(was.step === "" ? {} : { step: was.step }), name: was.name });
    }
    setWas({ ...draft, name: identity.name });
    setDraft({ ...draft, name: identity.name });
    setStanding("saved");
    setAnswer(result.saved);
    onSaved(result.saved);
  }

  async function remove() {
    if (was === null) return;
    const result = await binding.onRemove(was.scope, { when: was.when, ...(was.step === "" ? {} : { step: was.step }), name: was.name });
    if (result.ok) onRemoved();
    else {
      setStanding("refused");
      setSaid(result.said);
    }
  }

  const title = draft === null ? "Trigger" : draft.name === "" ? "New trigger" : draft.name;
  return (
    <Sheet kind="workflow-edit" open floating title={title} closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <div className="armada-wf-panel">
        {draft === null ? null : (
          <TriggerFields draft={draft} workflow={workflow} steps={steps} commands={binding.commands} onChange={change} />
        )}
        {standing === "refused" ? (
          <Alert tone="caution" title="Not saved">
            {said}
          </Alert>
        ) : null}
        {draft === null ? null : (
          <div className="armada-triggers__acts">
            <Button variant="primary" disabled={draft.with === ""} onClick={() => void save()}>
              {replaces ? "Replace" : keeping === undefined ? "Save" : "Keep"}
            </Button>
            {target.kind === "open" && was !== null ? (
              <Button variant="ghost" onClick={() => void remove()}>
                <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove trigger
              </Button>
            ) : null}
            {standing === "saved" && answer !== null ? <SavedMarks saved={answer} scope={draft.scope} /> : null}
          </div>
        )}
      </div>
    </Sheet>
  );
}

/** Whether a save lands somewhere other than where the Trigger was read from. */
function moved(was: TriggerDraft, now: TriggerDraft): boolean {
  return was.scope !== now.scope || was.when !== now.when || was.step !== now.step || (was.name !== "" && now.name !== "" && was.name !== now.name);
}

/** What a save answered, as marks: where it runs from, that it waits on `main`, that it is skipped here. */
function SavedMarks({ saved, scope }: { saved: TriggerSaved; scope: TriggerScope }) {
  return (
    <span className="armada-triggers__fired">
      <Mark icon={Check} label="Saved" hue="passed" />
      {saved.runs_from !== undefined && saved.runs_from !== scope ? (
        <Tooltip label={`Replaced by ${LEVEL[saved.runs_from].said.toLowerCase()}`}>
          <span className="armada-triggers__mark" role="img" aria-label={`Replaced by ${LEVEL[saved.runs_from].said.toLowerCase()}`}>
            {(() => {
              const { Glyph } = LEVEL[saved.runs_from!];
              return <Glyph size={12} strokeWidth={2} aria-hidden />;
            })()}
          </span>
        </Tooltip>
      ) : null}
      {saved.waits_for_main === true ? <WaitsMark /> : null}
      {saved.skipped === undefined ? null : <SkippedMark skipped={saved.skipped} />}
    </span>
  );
}

/** A firing's state: its glyph and hue, and what the tooltip says where Fleet said more. */
const STATE: Record<TriggerFiringState, { Glyph: LucideIcon; said: string; hue?: string }> = {
  pending: { Glyph: CircleDashed, said: "Pending" },
  skipped: { Glyph: Minus, said: "Skipped" },
  running: { Glyph: CircleDot, said: "Running", hue: "running" },
  passed: { Glyph: Check, said: "Passed", hue: "passed" },
  failed: { Glyph: X, said: "Failed", hue: "failed" },
  awaiting_owner: { Glyph: Eye, said: "Waiting on you", hue: "waiting" },
};

export function FiringMark({ trigger }: { trigger: { state: TriggerFiringState; skipped?: { said: string } | undefined; exit_code?: number | undefined } }) {
  const { Glyph, said, hue } = STATE[trigger.state];
  const label = trigger.state === "skipped" && trigger.skipped !== undefined ? trigger.skipped.said : trigger.state === "failed" && trigger.exit_code !== undefined ? `${said}, exit ${trigger.exit_code}` : said;
  return <Mark icon={Glyph} label={label} {...(hue === undefined ? {} : { hue })} pulsing={trigger.state === "running"} />;
}

/**
 * What a Job holds of its Triggers, a row each: the state as a mark, the name, the moment, and
 * the level that sets it. **A firing with a log line is a button on its name**, which goes to the line.
 */
export function FiredTriggers({ triggers, onOpenLog }: { triggers: readonly JobTrigger[]; onOpenLog?: (trigger: JobTrigger) => void }) {
  return (
    <ul className="armada-triggers" aria-label="Triggers">
      {triggers.map((one) => (
        <li key={`${one.when}|${one.step}|${one.name}`} className="armada-triggers__row armada-triggers__row--fired">
          <FiringMark trigger={one} />
          {one.log_at !== undefined && onOpenLog !== undefined ? (
            <Tooltip asChild label="Job log">
              <button type="button" className="armada-triggers__name armada-triggers__link" onClick={() => onOpenLog(one)}>
                {one.name}
              </button>
            </Tooltip>
          ) : (
            <span className="armada-triggers__name">{one.name}</span>
          )}
          <span className="armada-triggers__when">{whenSaid(one.when, one.step)}</span>
          <LevelMark level={one.level} />
        </li>
      ))}
    </ul>
  );
}

/** What a Job can be given between its steps. */
export type AddedKind = AddedRuns["kind"];

const ADDED_KIND: Record<AddedKind, string> = { script: "Script", skill: "Skill", drone: "Drone step" };

/** The `+` on a connector: a menu of what one Job can be given. It draws over the canvas, which is why it takes `portal`. */
export function AddStep({ label, onPick }: { label: string; onPick: (kind: AddedKind) => void }) {
  return (
    <span className="armada-triggers__add">
      <Tooltip label="Add a step to this Job only">
        <span>
          <DropdownMenu
            triggerLabel={label}
            icon={SquarePlus}
            portal
            entries={(Object.keys(ADDED_KIND) as AddedKind[]).map((id) => ({ kind: "item", id, label: ADDED_KIND[id] }))}
            onSelect={(id) => onPick(id as AddedKind)}
          />
        </span>
      </Tooltip>
    </span>
  );
}

/** What an added step runs, as its card names it. */
export const addedName = (runs: AddedRuns): string => (runs.kind === "script" ? runs.command : runs.kind === "skill" ? runs.skill : "Drone step");

/** What it runs, as its panel's field holds it: the Command, the skill, or the brief. */
const ranText = (runs: AddedRuns): string => (runs.kind === "script" ? runs.command : runs.kind === "skill" ? runs.skill : runs.brief);

/** An added step's firing state as its card's activity and hue. */
const ADDED_STATE: Record<TriggerFiringState, { activity: WorkflowStepCardProps["activity"]; token: string }> = {
  pending: { activity: "not_started", token: "--status-not-started" },
  skipped: { activity: "not_started", token: "--status-not-started" },
  running: { activity: "running", token: "--status-running" },
  passed: { activity: "advanced", token: "--step-advanced" },
  failed: { activity: "failed", token: "--step-failed" },
  awaiting_owner: { activity: "not_started", token: "--step-waiting" },
};

/**
 * An added step as a card: a dashed accent edge and the `webhook` mark, so it reads apart from the
 * workflow's own. **One Fleet recorded skipped wears the skipped mark**, with its reason on its chip.
 */
export function addedCard(one: AddedStep, selected: boolean, onOpen: () => void): WorkflowStepCardProps {
  const { activity, token } = ADDED_STATE[one.state];
  const fails = [one.block ? "blocks" : "", one.repair ? "repairs" : ""].filter((word) => word !== "").join(", ");
  const skipped = one.state === "skipped" && one.skipped !== undefined;
  return {
    kind: "step",
    name: addedName(one.runs),
    ...(one.runs.kind === "drone" ? {} : { nameIsAnIdentifier: true }),
    activity,
    mark: { icon: skipped ? Minus : Webhook, token },
    said: `Added to this Job only, ${STATE[one.state].said.toLowerCase()}`,
    added: true,
    facts: [
      { value: ADDED_KIND[one.runs.kind].toLowerCase() },
      ...(skipped ? [{ value: "skipped", hint: one.skipped!.said }] : []),
      ...(fails === "" ? [] : [{ value: fails }]),
    ],
    selected,
    onOpen,
  };
}

/**
 * The fields of a step added to one Job. **Editable until it is added**, at the gate or in the
 * panel that adds it; after that it is read, because Fleet has no edit for one: what a person can
 * still do is take it off before it fires, and keep it for every Job.
 */
export function AddedFields({
  one,
  commands,
  editable,
  onChange,
  onKeep,
  onRemove,
}: {
  one: AddedStep;
  commands: readonly string[];
  editable: boolean;
  onChange?: (next: Partial<Pick<AddedStep, "runs" | "block" | "repair">>) => void;
  onKeep?: () => void;
  onRemove?: () => void;
}) {
  const { runs } = one;
  const held = runs.kind !== "script" || runs.command === "" || commands.includes(runs.command) ? commands : [...commands, runs.command];
  const change = onChange ?? (() => undefined);
  return (
    <>
      <span className="armada-triggers__fact">{`${ADDED_KIND[runs.kind]}, this Job only`}</span>
      <span className="armada-triggers__fact">{whenSaid(one.when, one.step)}</span>
      {editable ? null : (
        <span className="armada-triggers__fired">
          <FiringMark trigger={one} />
          {one.kept === undefined ? null : <span className="armada-triggers__fact">{`Kept: ${LEVEL[one.kept].said.toLowerCase()}`}</span>}
        </span>
      )}
      {!editable ? (
        runs.kind === "drone" ? (
          <span className="armada-triggers__fact">{runs.brief}</span>
        ) : (
          <span className="armada-triggers__fact">{ranText(runs)}</span>
        )
      ) : runs.kind === "drone" ? (
        <Textarea label="Brief" rows={4} value={runs.brief} onChange={(event) => change({ runs: { kind: "drone", brief: event.target.value } })} />
      ) : runs.kind === "script" ? (
        <Select label="Script" value={runs.command} onChange={(event) => change({ runs: { kind: "script", command: event.target.value } })}>
          <option value="" />
          {held.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </Select>
      ) : (
        <Input label="Skill" mono value={runs.skill} onChange={(event) => change({ runs: { kind: "skill", skill: event.target.value } })} />
      )}
      <Fails block={one.block} repair={one.repair} disabled={!editable} onChange={change} />
      {onKeep === undefined && onRemove === undefined ? null : (
        <div className="armada-triggers__acts">
          {onKeep === undefined || runs.kind === "drone" || one.kept !== undefined ? null : (
            <Button variant="secondary" onClick={onKeep}>
              Keep for every Job
            </Button>
          )}
          {onRemove === undefined ? null : (
            <Button variant="ghost" onClick={onRemove}>
              <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove step
            </Button>
          )}
        </div>
      )}
    </>
  );
}
