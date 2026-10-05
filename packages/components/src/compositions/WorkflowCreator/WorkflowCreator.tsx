import { useMemo, useState } from "react";
import { Ban, Briefcase, Check, FolderGit2, MessageSquare, Package, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { WorkflowCanvas, type WorkflowCanvasEdge, type WorkflowCanvasNode } from "../WorkflowCanvas/WorkflowCanvas";
import {
  blankDefinition,
  blankStep,
  CHECKS,
  EVIDENCE,
  fileOf,
  KIT,
  refusalsOf,
  resolve,
  SOURCE_RANK,
  textOf,
  type Definition,
  type Entry,
  type Gate,
  type Refusal,
  type Resolved,
  type Source,
  type Step,
} from "./def";

/**
 * The Workflow creator: the workflows Fleet resolved for this repository as a
 * list of rows, and the one picked drawn as the graph it is — steps as nodes
 * on the Job's own canvas, a step that sends work back as a back edge. A step
 * or the workflow's own settings open in the app's one overlay panel.
 *
 * **A mock.** It reads nothing and writes nothing: `entries` and `definitions`
 * are the whole of what it knows.
 */
export type WorkflowCreatorProps = {
  /** The Manifest the window is in: where a new definition is written until another is picked. */
  repository: string;
  /** Every Manifest a definition may be written to, beside Kit. */
  manifests: readonly string[];
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

/** What is open in the panel: the workflow's own settings, or one step. */
type Panel = "workflow" | number | null;

const STANDING: Record<Standing, string> = { draft: "Draft", refused: "Refused", saved: "Saved" };

/** What each place is, said once on its mark. */
const PLACE: Record<Source, { Glyph: LucideIcon; said: string }> = {
  carried: { Glyph: Package, said: "Ships with Armada" },
  kit: { Glyph: Briefcase, said: "Your machine" },
  repository: { Glyph: FolderGit2, said: "This repository" },
};

function copy<T>(value: T): T {
  return structuredClone(value);
}

/** One card's height with a row under its name, and what each further row adds. `workflow-canvas.ts`'s own numbers. */
const STEP_APART = 112;
const ROW = 28;

const canvasId = (at: number) => `step:${at}`;

function gateWords(gate: Gate): string | undefined {
  if (gate.repository) return "repository decides";
  const ticked = [gate.checks ? "checks" : null, gate.judge ? "judge" : null, gate.you ? "you" : null].filter(
    (one) => one !== null,
  );
  return ticked.length === 0 ? undefined : ticked.join(" · ");
}

/** The definition as the canvas draws it: a spine down, and a back edge wherever a step sends work back. */
function graphOf(def: Definition, marked: ReadonlyMap<number, string[]>, panel: Panel, onOpen: (at: number) => void) {
  let y = 0;
  const nodes: WorkflowCanvasNode[] = def.steps.map((step, at) => {
    const needs = (marked.get(at) ?? []).map((says) => ({ says, tone: "waiting" as const }));
    const gate = gateWords(step.gate);
    const facts = [{ value: step.evidence }, ...(step.check === "none" ? [] : [{ value: step.check }])];
    const rows = needs.length + (gate === undefined ? 0 : 1);
    const here = y;
    y += STEP_APART + (Math.max(rows, 1) - 1) * ROW;
    return {
      id: canvasId(at),
      position: { x: 0, y: here },
      card: {
        kind: "step",
        name: step.id === "" ? `step ${at + 1}` : step.id,
        nameIsAnIdentifier: step.id !== "",
        activity: "not_started",
        said: `step ${at + 1}`,
        ordinal: at + 1,
        facts,
        needs,
        ...(gate === undefined ? {} : { gate }),
        selected: panel === at,
        onOpen: () => onOpen(at),
      },
    };
  });
  const edges: WorkflowCanvasEdge[] = [];
  def.steps.forEach((step, at) => {
    if (at > 0) edges.push({ id: `leads:${at}`, source: canvasId(at - 1), target: canvasId(at), kind: "leads" });
    const to = step.returnsTo === "" ? -1 : def.steps.findIndex((one) => one.id === step.returnsTo);
    if (to !== -1 && to < at) {
      edges.push({
        id: `returns:${at}`,
        source: canvasId(at),
        target: canvasId(to),
        kind: "returns",
        label: `up to ${step.iterationCap} passes`,
      });
    }
  });
  return { nodes, edges };
}

export function WorkflowCreator({
  repository,
  manifests,
  entries: given,
  definitions: known,
  onDiscuss,
}: WorkflowCreatorProps) {
  const [entries, setEntries] = useState<readonly Entry[]>(given);
  const [definitions, setDefinitions] = useState<Readonly<Record<string, Definition>>>(known);
  const [open, setOpen] = useState<Open | null>(null);
  const [picked, setPicked] = useState<Entry | null>(null);
  const [panel, setPanel] = useState<Panel>(null);

  const listed: Resolved[] = resolve(entries).sort(
    (a, b) => a.id.localeCompare(b.id) || SOURCE_RANK[b.source] - SOURCE_RANK[a.source],
  );

  function begin(entry: Resolved) {
    setPicked(entry);
    setPanel(null);
    const held = definitions[entry.key];
    if (entry.leftOut !== undefined || held === undefined) {
      setOpen(null);
      return;
    }
    const def = copy(held);
    // A carried definition is edited as a copy, written to the Manifest the window is in.
    if (entry.source === "carried") def.scope = repository;
    if (entry.source === "kit") def.scope = KIT;
    if (entry.source === "repository") def.scope = repository;
    setOpen({ from: entry.key, def, standing: "draft", refusals: [], handed: false });
  }

  function change(next: (def: Definition) => Definition) {
    setOpen((was) =>
      was === null ? was : { ...was, def: next(was.def), standing: "draft", refusals: [], handed: false },
    );
  }

  function save() {
    if (open === null) return;
    const { def } = open;
    const source: Source = def.scope === KIT ? "kit" : "repository";
    const key = `${source}/${def.id}`;
    const here = def.scope === KIT || def.scope === repository;
    const taken = entries
      .filter((one) => here && one.source === source && one.key !== open.from && one.leftOut === undefined)
      .map((one) => one.id);
    const refusals = refusalsOf(def, taken);
    if (refusals.length > 0) {
      setOpen({ ...open, standing: "refused", refusals });
      return;
    }
    if (here) {
      const entry: Entry = { key, id: def.id, source, file: `${def.id}.json` };
      setEntries((was) => (was.some((one) => one.key === key) ? was.map((one) => (one.key === key ? entry : one)) : [...was, entry]));
      setDefinitions((was) => ({ ...was, [key]: copy(def) }));
      setPicked(entry);
    }
    setOpen({ ...open, from: here ? key : open.from, standing: "saved", refusals: [], handed: open.handed });
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
            onClick={() => {
              setPicked(null);
              setPanel(null);
              setOpen({ def: blankDefinition(repository), standing: "draft", refusals: [], handed: false });
            }}
          >
            New workflow
          </Button>
        </div>
        <ul className="armada-wf-rows" aria-label="Workflow files">
          {listed.map((entry) => (
            <Row key={entry.key} entry={entry} on={picked?.key === entry.key} onOpen={() => begin(entry)} />
          ))}
        </ul>
      </section>
      {picked?.leftOut !== undefined ? (
        <section className="armada-wf-left" aria-label={`${picked.id}, left out`}>
          <span className="armada-wf-left__file">{picked.file}</span>
          <span className="armada-wf-left__why">{picked.leftOut}</span>
        </section>
      ) : open === null ? null : (
        <Stage
          open={open}
          repository={repository}
          manifests={manifests}
          entries={entries}
          panel={panel}
          onPanel={setPanel}
          onChange={change}
          onSave={save}
          onDiscuss={onDiscuss === undefined ? undefined : discuss}
        />
      )}
    </div>
  );
}

function Row({ entry, on, onOpen }: { entry: Resolved; on: boolean; onOpen: () => void }) {
  const leftOut = entry.leftOut !== undefined;
  const { Glyph, said } = leftOut ? { Glyph: Ban, said: `Cannot run. ${entry.leftOut}` } : PLACE[entry.source];
  const where = leftOut ? `${PLACE[entry.source].said}. ${said}` : said;
  const tip = entry.overriddenBy === undefined ? where : `${where}. Overridden by ${PLACE[entry.overriddenBy].said.toLowerCase()}`;
  const label = `${entry.id}, ${entry.source}${leftOut ? ", left out" : entry.overriddenBy !== undefined ? ", overridden" : ""}`;
  return (
    <li className="armada-wf-row" data-state={leftOut ? "left-out" : entry.overriddenBy !== undefined ? "overridden" : undefined}>
      <button type="button" aria-label={label} aria-current={on || undefined} onClick={onOpen}>
        <Tooltip label={tip}>
          <span className="armada-wf-row__mark" role="img" aria-label={leftOut ? "Left out" : PLACE[entry.source].said}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="armada-wf-row__id">{entry.id}</span>
      </button>
    </li>
  );
}

function Stage({
  open,
  repository,
  manifests,
  entries,
  panel,
  onPanel,
  onChange,
  onSave,
  onDiscuss,
}: {
  open: Open;
  repository: string;
  manifests: readonly string[];
  entries: readonly Entry[];
  panel: Panel;
  onPanel: (panel: Panel) => void;
  onChange: (next: (def: Definition) => Definition) => void;
  onSave: () => void;
  onDiscuss: (() => void) | undefined;
}) {
  const { def, standing, refusals } = open;
  const ScopeGlyph = def.scope === KIT ? Briefcase : FolderGit2;
  const marked = useMemo(() => {
    const by = new Map<number, string[]>();
    for (const one of refusals) if (one.step !== undefined) by.set(one.step, [...(by.get(one.step) ?? []), one.why]);
    return by;
  }, [refusals]);
  const { nodes, edges } = useMemo(() => graphOf(def, marked, panel, onPanel), [def, marked, panel, onPanel]);
  const replaces = entries
    .filter((one) => one.id === def.id && one.leftOut === undefined && SOURCE_RANK[one.source] < SOURCE_RANK[def.scope === KIT ? "kit" : "repository"])
    .map((one) => PLACE[one.source].said);

  return (
    <section className="armada-wf-stage" data-standing={standing} aria-label="Workflow definition">
      <div className="armada-wf-stage__band">
        {standing === "saved" ? (
          <span className="armada-wf-stage__state" aria-hidden>
            <Check size={12} strokeWidth={2} />
          </span>
        ) : null}
        <span className="armada-wf-stage__eyebrow">{STANDING[standing]}</span>
        <span className="armada-wf-stage__name">{def.id}</span>
        <Tooltip label={def.scope === KIT ? "Kit" : def.scope}>
          <span className="armada-wf-stage__mark" role="img" aria-label={def.scope === KIT ? "Kit" : def.scope}>
            <ScopeGlyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        {open.handed ? (
          <Tooltip label="Handed to Helm">
            <span className="armada-wf-stage__mark" role="img" aria-label="Handed to Helm">
              <MessageSquare size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        ) : null}
      </div>

      <div className="armada-wf-stage__acts">
        <Button variant="secondary" onClick={() => onPanel("workflow")}>
          Settings
        </Button>
        <Button
          variant="secondary"
          onClick={() => {
            onChange((was) => ({ ...was, steps: [...was.steps, blankStep()] }));
            onPanel(def.steps.length);
          }}
        >
          Add step
        </Button>
        <span className="armada-wf-stage__end">
          <Button variant="ghost" disabled={onDiscuss === undefined} onClick={onDiscuss}>
            Discuss with Helm
          </Button>
          <Button variant="primary" onClick={onSave}>
            Save
          </Button>
        </span>
      </div>

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

      <div className="armada-wf-stage__canvas">
        <WorkflowCanvas nodes={nodes} edges={edges} label={`${def.id === "" ? "New workflow" : def.id} steps`} hangsFromTop runsDown />
      </div>

      {panel === null ? null : (
        <Sheet
          kind="workflow-edit"
          open
          floating
          title={panel === "workflow" ? (def.id === "" ? "Workflow" : def.id) : stepName(def.steps[panel], panel)}
          closeLabel="Close"
          closeBinding="Esc"
          onClose={() => onPanel(null)}
        >
          <div className="armada-wf-panel">
            {panel === "workflow" ? (
              <WorkflowFields
                def={def}
                repository={repository}
                manifests={manifests}
                replaces={replaces}
                invalid={refusals.some((one) => one.where === "id")}
                onChange={onChange}
              />
            ) : def.steps[panel] === undefined ? null : (
              <StepFields
                step={def.steps[panel]}
                at={panel}
                steps={def.steps}
                refused={marked.has(panel)}
                onChange={(next) =>
                  onChange((was) => ({ ...was, steps: was.steps.map((one, i) => (i === panel ? { ...one, ...next } : one)) }))
                }
                onRemove={() => {
                  onChange((was) => ({ ...was, steps: was.steps.filter((_, i) => i !== panel) }));
                  onPanel(null);
                }}
              />
            )}
          </div>
        </Sheet>
      )}
    </section>
  );
}

function stepName(step: Step | undefined, at: number): string {
  return step === undefined || step.id === "" ? `Step ${at + 1}` : step.id;
}

function WorkflowFields({
  def,
  repository,
  manifests,
  replaces,
  invalid,
  onChange,
}: {
  def: Definition;
  repository: string;
  manifests: readonly string[];
  replaces: readonly string[];
  invalid: boolean;
  onChange: (next: (def: Definition) => Definition) => void;
}) {
  return (
    <>
      <Input
        label="Workflow id"
        mono
        value={def.id}
        invalid={invalid}
        onChange={(event) => onChange((was) => ({ ...was, id: event.target.value }))}
      />
      <Select label="Scope" value={def.scope} onChange={(event) => onChange((was) => ({ ...was, scope: event.target.value }))}>
        <option value={KIT}>Kit</option>
        {manifests.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
      <span className="armada-wf-panel__fact">{fileOf(def, repository)}</span>
      {replaces.length === 0 ? null : (
        <span className="armada-wf-panel__fact">{`replaces ${replaces.join(", ").toLowerCase()}`}</span>
      )}
    </>
  );
}

function StepFields({
  step,
  at,
  steps,
  refused,
  onChange,
  onRemove,
}: {
  step: Step;
  at: number;
  steps: readonly Step[];
  refused: boolean;
  onChange: (next: Partial<Step>) => void;
  onRemove: () => void;
}) {
  const gate = (next: Partial<Gate>) => onChange({ gate: { ...step.gate, ...next } });
  const decided = step.gate.repository;
  return (
    <>
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
      <Input label="Judge question" mono value={step.judge} onChange={(event) => onChange({ judge: event.target.value })} />
      <div className="armada-wf-panel__gate" role="group" aria-label="Advances on">
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
      <Input
        label="Retries"
        mono
        type="number"
        min={0}
        value={step.retryLimit}
        onChange={(event) => onChange({ retryLimit: event.target.valueAsNumber })}
      />
      <Select label="Sends work back to" value={step.returnsTo} onChange={(event) => onChange({ returnsTo: event.target.value })}>
        <option value="" />
        {steps.slice(0, at).map((one, i) =>
          one.id === "" ? null : (
            <option key={i} value={one.id}>
              {one.id}
            </option>
          ),
        )}
      </Select>
      {step.returnsTo === "" ? null : (
        <Input
          label="Passes"
          mono
          type="number"
          min={1}
          value={step.iterationCap}
          onChange={(event) => onChange({ iterationCap: event.target.valueAsNumber })}
        />
      )}
      <div>
        <Button variant="ghost" onClick={onRemove}>
          <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove step
        </Button>
      </div>
    </>
  );
}
