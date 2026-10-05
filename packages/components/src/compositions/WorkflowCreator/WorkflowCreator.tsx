import { useState } from "react";
import { Briefcase, Check, FolderGit2, MessageSquare, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import {
  blankDefinition,
  blankStep,
  CHECKS,
  EVIDENCE,
  fileOf,
  refusalsOf,
  resolve,
  SOURCE_DIR,
  SOURCE_RANK,
  SOURCE_WORD,
  textOf,
  type Definition,
  type Entry,
  type Gate,
  type Refusal,
  type Resolved,
  type Scope,
  type Step,
  type Structure,
} from "./def";

/**
 * The Workflow creator: every workflow Fleet resolved for this repository, and
 * one of them open to edit. A bay per file in the list, banded by the place it
 * came from; the open definition is one frame whose band says where it stands,
 * drafted, refused or saved.
 *
 * **A mock.** It reads nothing and writes nothing: `entries` and `definitions`
 * are the whole of what it knows.
 */
export type WorkflowCreatorProps = {
  /** The repository's name, for what "this Manifest" points at. */
  repository: string;
  entries: readonly Entry[];
  /** The definition behind each editable entry, by its key. */
  definitions: Readonly<Record<string, Definition>>;
  /** Hand the draft to Helm, which can author one too. */
  onDiscuss?: (draft: string) => void;
};

type Standing = "draft" | "refused" | "saved";

type Open = {
  /** The file being edited. Absent for a new one. */
  from?: string;
  def: Definition;
  standing: Standing;
  refusals: Refusal[];
  handed: boolean;
};

const STANDING: Record<Standing, string> = { draft: "Draft", refused: "Refused", saved: "Saved" };

const SOURCE_GLYPH: Record<Scope, LucideIcon> = { kit: Briefcase, repository: FolderGit2 };

/** A place as a sentence names it: Kit keeps its capital, the others are common nouns. */
function named(source: Entry["source"]): string {
  return source === "kit" ? "Kit" : source;
}

function copy<T>(value: T): T {
  return structuredClone(value);
}

export function WorkflowCreator({ repository, entries: given, definitions: known, onDiscuss }: WorkflowCreatorProps) {
  const [entries, setEntries] = useState<readonly Entry[]>(given);
  const [definitions, setDefinitions] = useState<Readonly<Record<string, Definition>>>(known);
  const [open, setOpen] = useState<Open | null>(null);

  const listed: Resolved[] = resolve(entries).sort(
    (a, b) => a.id.localeCompare(b.id) || SOURCE_RANK[b.source] - SOURCE_RANK[a.source],
  );

  function begin(entry: Resolved) {
    const held = definitions[entry.key];
    if (held === undefined) return;
    setOpen({ from: entry.key, def: copy(held), standing: "draft", refusals: [], handed: false });
  }

  function change(next: (def: Definition) => Definition) {
    setOpen((was) =>
      was === null ? was : { ...was, def: next(was.def), standing: "draft", refusals: [], handed: false },
    );
  }

  function save() {
    if (open === null) return;
    const { def } = open;
    const key = `${def.scope}/${def.id}`;
    const taken = entries
      .filter((one) => one.source === def.scope && one.key !== open.from && one.leftOut === undefined)
      .map((one) => one.id);
    const refusals = refusalsOf(def, taken);
    if (refusals.length > 0) {
      setOpen({ ...open, standing: "refused", refusals });
      return;
    }
    const entry: Entry = { key, id: def.id, source: def.scope, file: `${def.id}.json`, structure: def.structure };
    setEntries((was) => (was.some((one) => one.key === key) ? was.map((one) => (one.key === key ? entry : one)) : [...was, entry]));
    setDefinitions((was) => ({ ...was, [key]: copy(def) }));
    setOpen({ ...open, from: key, standing: "saved", refusals: [], handed: open.handed });
  }

  function discuss() {
    if (open === null) return;
    onDiscuss?.(textOf(open.def));
    setOpen({ ...open, handed: true });
  }

  return (
    <div className="armada-wf">
      <section className="armada-wf__list" aria-label="Workflows">
        <div>
          <Button
            variant="secondary"
            onClick={() => setOpen({ def: blankDefinition(), standing: "draft", refusals: [], handed: false })}
          >
            New workflow
          </Button>
        </div>
        <ul className="armada-wf-bays" aria-label="Workflow files">
          {listed.map((entry) => (
            <Bay key={entry.key} entry={entry} on={open?.from === entry.key} onOpen={() => begin(entry)} />
          ))}
        </ul>
      </section>
      {open === null ? null : (
        <Frame
          open={open}
          repository={repository}
          replaces={replacedBy(open.def, entries)}
          onChange={change}
          onSave={save}
          onDiscuss={onDiscuss === undefined ? undefined : discuss}
        />
      )}
    </div>
  );
}

/** What the open definition would shadow once written: lower places holding its id. */
function replacedBy(def: Definition, entries: readonly Entry[]): Entry["source"][] {
  return entries
    .filter((one) => one.id === def.id && one.leftOut === undefined && SOURCE_RANK[one.source] < SOURCE_RANK[def.scope])
    .map((one) => one.source);
}

function Bay({ entry, on, onOpen }: { entry: Resolved; on: boolean; onOpen: () => void }) {
  const Glyph = entry.source === "carried" ? null : SOURCE_GLYPH[entry.source];
  const label = `${entry.id}, ${SOURCE_WORD[entry.source].toLowerCase()}${
    entry.leftOut !== undefined ? ", left out" : entry.overriddenBy !== undefined ? ", overridden" : ""
  }`;
  const bay = entry.leftOut !== undefined ? "left-out" : entry.overriddenBy !== undefined ? "overridden" : "active";
  const place = (
    <span className="armada-wf-bay__file">
      {entry.source === "carried" ? SOURCE_DIR.carried : `${SOURCE_DIR[entry.source]}/${entry.file}`}
    </span>
  );

  return (
    <li className="armada-wf-bay" data-bay={bay} data-on={on ? "" : undefined} aria-label={label}>
      <div className="armada-wf-bay__band">
        {Glyph === null ? null : (
          <Tooltip label={SOURCE_DIR[entry.source]}>
            <span className="armada-wf-bay__mark" role="img" aria-label={SOURCE_WORD[entry.source]}>
              <Glyph size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        )}
        <span className="armada-wf-bay__eyebrow" aria-hidden>
          {entry.leftOut !== undefined ? "Left out" : SOURCE_WORD[entry.source]}
        </span>
        <span className="armada-wf-bay__structure">{entry.structure}</span>
      </div>
      <div className="armada-wf-bay__body">
        {entry.leftOut !== undefined ? (
          <span className="armada-wf-bay__id">{entry.id}</span>
        ) : (
          <button type="button" className="armada-wf-bay__id" aria-current={on || undefined} onClick={onOpen}>
            {entry.id}
          </button>
        )}
        {place}
        {entry.leftOut === undefined ? null : <span className="armada-wf-bay__why">{entry.leftOut}</span>}
        {entry.overriddenBy === undefined ? null : (
          <span className="armada-wf-bay__over">{`overridden by ${named(entry.overriddenBy)}`}</span>
        )}
      </div>
    </li>
  );
}

function Frame({
  open,
  repository,
  replaces,
  onChange,
  onSave,
  onDiscuss,
}: {
  open: Open;
  repository: string;
  replaces: Entry["source"][];
  onChange: (next: (def: Definition) => Definition) => void;
  onSave: () => void;
  onDiscuss: (() => void) | undefined;
}) {
  const { def, standing, refusals } = open;
  const ScopeGlyph = SOURCE_GLYPH[def.scope];
  const marked = new Set(refusals.flatMap((one) => (one.step === undefined ? [] : [one.step])));

  function setStep(at: number, next: Partial<Step>) {
    onChange((was) => ({ ...was, steps: was.steps.map((one, i) => (i === at ? { ...one, ...next } : one)) }));
  }

  return (
    <section className="armada-wf-frame" data-standing={standing} aria-label="Workflow definition">
      <div className="armada-wf-frame__band">
        {standing === "saved" ? (
          <span className="armada-wf-frame__state" aria-hidden>
            <Check size={12} strokeWidth={2} />
          </span>
        ) : null}
        <span className="armada-wf-frame__eyebrow">{STANDING[standing]}</span>
        <span className="armada-wf-frame__name">{def.id}</span>
        <Tooltip label={fileOf(def)}>
          <span className="armada-wf-frame__mark" role="img" aria-label={def.scope === "kit" ? "Kit" : "This Manifest"}>
            <ScopeGlyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        {open.handed ? (
          <Tooltip label="Handed to Helm">
            <span className="armada-wf-frame__mark" role="img" aria-label="Handed to Helm">
              <MessageSquare size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : null}
      </div>

      <div className="armada-wf-frame__body">
        {refusals.length === 0 ? null : (
          <Alert tone="caution" title="Not saved">
            <ul className="armada-wf-refusals" aria-label="Refusals">
              {refusals.map((one, i) => (
                <li key={i}>
                  <code>{one.where}</code>
                  <span>{one.why}</span>
                </li>
              ))}
            </ul>
          </Alert>
        )}

        <div className="armada-wf-head">
          <Input
            label="Workflow id"
            mono
            value={def.id}
            invalid={refusals.some((one) => one.where === "id")}
            onChange={(event) => onChange((was) => ({ ...was, id: event.target.value }))}
          />
          <RadioGroup label="Structure">
            {(["linear", "loop"] as const satisfies readonly Structure[]).map((one) => (
              <Radio
                key={one}
                name="workflow-structure"
                checked={def.structure === one}
                onChange={() => onChange((was) => ({ ...was, structure: one }))}
              >
                {one === "linear" ? "Sequence" : "Loop"}
              </Radio>
            ))}
          </RadioGroup>
          <RadioGroup label="Scope">
            <Tooltip label={`${repository}/${SOURCE_DIR.repository}`}>
              <span>
                <Radio
                  name="workflow-scope"
                  checked={def.scope === "repository"}
                  onChange={() => onChange((was) => ({ ...was, scope: "repository" }))}
                >
                  This Manifest
                </Radio>
              </span>
            </Tooltip>
            <Tooltip label={SOURCE_DIR.kit}>
              <span>
                <Radio
                  name="workflow-scope"
                  checked={def.scope === "kit"}
                  onChange={() => onChange((was) => ({ ...was, scope: "kit" }))}
                >
                  Kit
                </Radio>
              </span>
            </Tooltip>
          </RadioGroup>
          {def.structure === "loop" ? (
            <Input
              label="Iteration cap"
              mono
              type="number"
              min={1}
              value={def.iterationCap}
              invalid={refusals.some((one) => one.where === "iteration_cap")}
              onChange={(event) => onChange((was) => ({ ...was, iterationCap: event.target.valueAsNumber }))}
            />
          ) : null}
        </div>

        <ol className="armada-wf-steps" aria-label="Steps">
          {def.steps.map((step, at) => (
            <StepCard
              key={at}
              step={step}
              at={at}
              steps={def.steps}
              loop={def.structure === "loop"}
              refused={marked.has(at)}
              onChange={(next) => setStep(at, next)}
              onRemove={() => onChange((was) => ({ ...was, steps: was.steps.filter((_, i) => i !== at) }))}
            />
          ))}
        </ol>

        <div className="armada-wf-foot">
          <Tooltip label={fileOf(def)}>
            <span className="armada-wf-foot__file">{fileOf(def)}</span>
          </Tooltip>
          {replaces.length === 0 ? null : (
            <span className="armada-wf-foot__file">{`replaces ${replaces.map(named).join(", ")}`}</span>
          )}
          <span className="armada-wf-foot__acts">
            <Button variant="secondary" onClick={() => onChange((was) => ({ ...was, steps: [...was.steps, blankStep()] }))}>
              Add step
            </Button>
            <Button variant="ghost" disabled={onDiscuss === undefined} onClick={onDiscuss}>
              Discuss with Helm
            </Button>
            <Button variant="primary" onClick={onSave}>
              Save
            </Button>
          </span>
        </div>
      </div>
    </section>
  );
}

function StepCard({
  step,
  at,
  steps,
  loop,
  refused,
  onChange,
  onRemove,
}: {
  step: Step;
  at: number;
  steps: readonly Step[];
  loop: boolean;
  refused: boolean;
  onChange: (next: Partial<Step>) => void;
  onRemove: () => void;
}) {
  const n = at + 1;
  const gate = (next: Partial<Gate>) => onChange({ gate: { ...step.gate, ...next } });
  const decided = step.gate.repository;
  return (
    <li className="armada-wf-step" data-refused={refused ? "" : undefined} aria-label={`Step ${n}`}>
      <div className="armada-wf-step__band">
        <span className="armada-wf-step__order" aria-hidden>
          {n}
        </span>
        <span className="armada-wf-step__name">{step.id}</span>
        <Tooltip label="Remove step">
          <button type="button" className="armada-wf-step__act" aria-label={`Remove step ${n}`} onClick={onRemove}>
            <Trash2 size={12} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      </div>
      <div className="armada-wf-step__body">
        <Input
          label="Step id"
          mono
          value={step.id}
          invalid={refused && step.id === ""}
          onChange={(event) => onChange({ id: event.target.value })}
        />
        <Select label="Evidence" value={step.evidence} onChange={(event) => onChange({ evidence: event.target.value as Step["evidence"] })}>
          {EVIDENCE.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
        <Select label="Check" value={step.check} onChange={(event) => onChange({ check: event.target.value as Step["check"] })}>
          {CHECKS.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
        <Input
          label="Retries"
          mono
          type="number"
          min={0}
          value={step.retryLimit}
          onChange={(event) => onChange({ retryLimit: event.target.valueAsNumber })}
        />
        <div className="armada-wf-step__wide">
          <Input label="Judge question" mono value={step.judge} onChange={(event) => onChange({ judge: event.target.value })} />
        </div>
        <div className="armada-wf-step__gate" role="group" aria-label={`Step ${n} advances on`}>
          <Checkbox checked={step.gate.checks} disabled={decided} onChange={(event) => gate({ checks: event.target.checked })}>
            Checks
          </Checkbox>
          <Checkbox checked={step.gate.judge} disabled={decided} onChange={(event) => gate({ judge: event.target.checked })}>
            Judge
          </Checkbox>
          <Checkbox checked={step.gate.you} disabled={decided} onChange={(event) => gate({ you: event.target.checked })}>
            You
          </Checkbox>
          <Checkbox
            checked={decided}
            onChange={(event) =>
              gate(event.target.checked ? { repository: true, checks: false, judge: false, you: false } : { repository: false })
            }
          >
            Repository decides
          </Checkbox>
        </div>
        {loop ? (
          <Select label="Returns to" value={step.returnsTo} onChange={(event) => onChange({ returnsTo: event.target.value })}>
            <option value="" />
            {steps.map((one, i) =>
              one.id === "" || i === at ? null : (
                <option key={i} value={one.id}>
                  {one.id}
                </option>
              ),
            )}
          </Select>
        ) : null}
      </div>
    </li>
  );
}
