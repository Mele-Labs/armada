import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Ban, Briefcase, Check, FileCheck, FolderGit2, MessageSquare, Package, Scale, ShieldCheck, Trash2, UserCheck, Webhook, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { WorkflowCanvas, type WorkflowCanvasEdge, type WorkflowCanvasNode } from "../WorkflowCanvas/WorkflowCanvas";
import type { WorkflowStepBand, WorkflowStepDetail } from "../WorkflowStepCard/WorkflowStepCard";
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
  type Definition,
  type Entry,
  type Gate,
  type ManifestOption,
  type Read,
  type Refusal,
  type Resolved,
  type Saved,
  type Source,
  type Step,
} from "./def";
import { writeDefinition } from "./json";
import { TriggerRows, TriggerSheet, type TriggerTarget } from "../WorkflowTriggers/WorkflowTriggers";
import { firingAt, identityKey, type TriggersBinding } from "../WorkflowTriggers/triggers";
import type { TriggerLevel, TriggerMoment, TriggerSaved, TriggerSummary } from "@armada/protocol";

/**
 * The Workflow creator: the workflows Fleet resolved for this repository as a
 * list of rows, and the one picked drawn as the graph it is — steps as nodes
 * on the Job's own canvas, a step that sends work back as a back edge. A step
 * or the workflow's own settings open in the app's one overlay panel.
 *
 * It reads and writes nothing itself: `onRead` answers a row's definition and
 * `onSave` writes one, and what Fleet says of a save is shown as it said it.
 */
export type WorkflowCreatorProps = {
  /** The id of the Manifest the window is in: where a new definition is written until another is picked. */
  repository: string;
  /** Every Manifest a definition may be written to, beside Kit. */
  manifests: readonly ManifestOption[];
  entries: readonly Entry[];
  /** One file's definition, for a row's graph and for opening it. */
  onRead: (entry: Entry) => Promise<Read>;
  /** Write the draft. `overwrite` is sent only once Fleet has said a definition is already there. */
  onSave: (def: Definition, overwrite: boolean) => Promise<Saved>;
  /** Hand the draft to Helm, which can author one too. */
  onDiscuss?: (draft: string) => void;
  /** The repository's Triggers and what can be done to them. Absent draws none. */
  triggers?: TriggersBinding;
};

type Standing = "draft" | "refused" | "saved";

type Open = {
  /** The file being edited. Absent for a new one. */
  from?: string;
  def: Definition;
  standing: Standing;
  refusals: Refusal[];
  handed: boolean;
  /** Fleet said a definition of this id is there, so the next Save replaces it. */
  replaces?: true;
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

/** A card's height with one row under its name, a band over it, and what each further row adds. `workflow-canvas.ts`'s numbers. */
const STEP_APART = 112;
const BAND = 28;
const ROW = 28;

const canvasId = (at: number) => `step:${at}`;

/**
 * What a step IS, as its band: **how it advances**, since a definition has no
 * run to colour. The strongest setting picks the hue (you, then Judge, then
 * Checks); every ticked setting keeps its own mark and its word in the label,
 * so the meaning holds without the colour. The hues are existing status
 * aliases chosen for this mock, and the owner's to change.
 */
const GATE: Record<"checks" | "judge" | "you" | "repository" | "auto", { icon: LucideIcon; said: string; token: string }> = {
  checks: { icon: ShieldCheck, said: "Checks", token: "--status-running" },
  judge: { icon: Scale, said: "Judge", token: "--status-rejected" },
  you: { icon: UserCheck, said: "You", token: "--status-awaiting-review" },
  repository: { icon: FolderGit2, said: "Repository decides", token: "--accent" },
  auto: { icon: Zap, said: "Submission only", token: "--status-not-started" },
};

export function bandOf(step: Step, refused: boolean): WorkflowStepBand {
  const g = step.gate;
  const lead = g.repository ? "repository" : g.you ? "you" : g.judge ? "judge" : g.checks ? "checks" : "auto";
  const tag = step.id === "" ? undefined : step.id;
  if (refused) return { ...(tag === undefined ? {} : { tag }), token: "--notice-caution", look: "hatched" };
  return { ...(tag === undefined ? {} : { tag }), token: GATE[lead].token, look: lead === "auto" ? "dashed" : "solid" };
}

/** What the step is, a labelled line each: its evidence, then every way it advances. */
export function detailsOf(step: Step, triggers: readonly TriggerSummary[] = []): WorkflowStepDetail[] {
  const g = step.gate;
  const rows: WorkflowStepDetail[] = step.evidence === "" ? [] : [{ icon: FileCheck, label: "Evidence", value: step.evidence }];
  if (g.repository) rows.push({ icon: GATE.repository.icon, label: GATE.repository.said });
  else {
    if (g.checks) rows.push({ icon: GATE.checks.icon, label: GATE.checks.said, value: step.check === "none" ? "none named" : step.check });
    if (g.judge) rows.push({ icon: GATE.judge.icon, label: GATE.judge.said, value: step.judge.trim() === "" ? "no question" : step.judge });
    if (g.you) rows.push({ icon: GATE.you.icon, label: GATE.you.said });
    if (!g.checks && !g.judge && !g.you) rows.push({ icon: GATE.auto.icon, label: GATE.auto.said });
  }
  // Triggers fire at a moment, so each moment is a line of its own after how the step advances.
  const fires = (when: TriggerMoment, label: string) => {
    const named = triggers.filter((one) => one.when === when).map((one) => one.name);
    if (named.length > 0) rows.push({ icon: Webhook, label, value: named.join(", ") });
  };
  fires("pr_opened", "PR opened");
  fires("step_starts", "Triggers on start");
  fires("step_passes", "Triggers on pass");
  return rows;
}

/** The title a step reads as: its id without the underscores. */
const titleOf = (id: string, at: number) => (id === "" ? `Step ${at + 1}` : id.charAt(0).toUpperCase() + id.slice(1).replaceAll("_", " "));

/** The definition as the canvas draws it: a spine down, and a back edge wherever a step sends work back. */
function graphOf(def: Definition, marked: ReadonlyMap<number, string[]>, panel: Panel, onOpen: (at: number) => void, triggersAt: (step: Step) => TriggerSummary[]) {
  let y = 0;
  const nodes: WorkflowCanvasNode[] = def.steps.map((step, at) => {
    const needs = (marked.get(at) ?? []).map((says) => ({ says, tone: "waiting" as const }));
    const back = step.returnsTo !== "" && def.steps.findIndex((one) => one.id === step.returnsTo) < at && def.steps.some((one) => one.id === step.returnsTo);
    const details = detailsOf(step, triggersAt(step));
    const rows = details.length + needs.length + (back ? 1 : 0);
    const here = y;
    y += STEP_APART + BAND + (rows - 1) * ROW + 8;
    return {
      id: canvasId(at),
      position: { x: 0, y: here },
      card: {
        kind: "step",
        name: titleOf(step.id, at),
        activity: "not_started",
        said: `step ${at + 1}`,
        ordinal: at + 1,
        details,
        needs,
        band: bandOf(step, needs.length > 0),
        ...(back ? { action: `returns to ${step.returnsTo}`, returns: true } : {}),
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

export function WorkflowCreator({ repository, manifests, entries, onRead, onSave, onDiscuss, triggers }: WorkflowCreatorProps) {
  const [definitions, setDefinitions] = useState<Readonly<Record<string, Definition>>>({});
  const [open, setOpen] = useState<Open | null>(null);
  const [picked, setPicked] = useState<Entry | null>(null);
  const [panel, setPanel] = useState<Panel>(null);
  // The Trigger open in its own panel, and what each save answered, for the row's `waits_for_main`.
  const [triggerEdit, setTriggerEdit] = useState<{ target: TriggerTarget; at: number } | null>(null);
  const [answered, setAnswered] = useState<ReadonlyMap<string, TriggerSaved>>(new Map());
  const asked = useRef(new Set<string>());
  const opening = useRef(0);

  // Every row draws its graph, so every definition is read once, as the list arrives.
  useEffect(() => {
    for (const entry of entries) {
      if (entry.leftOut !== undefined || asked.current.has(entry.key)) continue;
      asked.current.add(entry.key);
      void onRead(entry).then((read) => {
        if (read.ok) setDefinitions((was) => ({ ...was, [entry.key]: read.def }));
      });
    }
  }, [entries, onRead]);

  const listed: Resolved[] = resolve(entries).sort(
    (a, b) => a.id.localeCompare(b.id) || SOURCE_RANK[b.source] - SOURCE_RANK[a.source],
  );

  async function begin(entry: Resolved) {
    const turn = ++opening.current;
    setPicked(entry);
    setPanel(null);
    setOpen(null);
    if (entry.leftOut !== undefined) return;
    const read = definitions[entry.key] === undefined ? await onRead(entry) : ({ ok: true, def: definitions[entry.key]! } as const);
    if (turn !== opening.current) return;
    // A file Fleet holds and this cannot draw says why, as a left-out one does.
    if (!read.ok) {
      setPicked({ ...entry, leftOut: read.said });
      return;
    }
    const def = copy(read.def);
    // A carried definition is edited as a copy, written to the Manifest the window is in.
    def.scope = entry.source === "kit" ? KIT : repository;
    setOpen({ from: entry.key, def, standing: "draft", refusals: [], handed: false });
  }

  // One overlay panel at a time: a step, the workflow's settings, or a Trigger.
  const openPanel = useCallback((next: Panel) => {
    setTriggerEdit(null);
    setPanel(next);
  }, []);

  function openTrigger(target: TriggerTarget) {
    setPanel(null);
    setTriggerEdit((was) => ({ target, at: (was?.at ?? 0) + 1 }));
  }

  function change(next: (def: Definition) => Definition) {
    setOpen((was) => {
      if (was === null) return was;
      const { replaces: _asked, ...kept } = was;
      return { ...kept, def: next(was.def), standing: "draft", refusals: [], handed: false };
    });
  }

  async function save() {
    if (open === null) return;
    const { def } = open;
    const source: Source = def.scope === KIT ? "kit" : "repository";
    const key = `${source}/${def.id}`;
    const here = def.scope === KIT || def.scope === repository;
    const refusals = refusalsOf(def);
    if (refusals.length > 0) {
      setOpen({ ...open, standing: "refused", refusals });
      return;
    }
    // Saving the file that was opened replaces it, which is the point; any other id asks first.
    const answer = await onSave(def, open.from === key || open.replaces === true);
    if (!answer.ok) {
      setOpen({
        ...open,
        standing: "refused",
        refusals: [{ where: "save", why: answer.said, field: "" }],
        handed: open.handed,
        ...(answer.exists === true ? { replaces: true as const } : {}),
      });
      return;
    }
    if (here) {
      setDefinitions((was) => ({ ...was, [key]: copy(def) }));
      setPicked({ key, id: def.id, source, file: `${def.id}.json` });
    }
    setOpen({ ...open, from: here ? key : open.from, standing: "saved", refusals: [], handed: open.handed });
  }

  function discuss() {
    if (open === null) return;
    onDiscuss?.(writeDefinition(open.def));
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
            <Row key={entry.key} entry={entry} def={definitions[entry.key]} on={picked?.key === entry.key} onOpen={() => void begin(entry)} />
          ))}
        </ul>
        {triggers === undefined ? null : (
          <section className="armada-triggers-card" aria-label="Triggers on every workflow">
            <div className="armada-triggers-card__band">Every workflow</div>
            <div className="armada-triggers-card__body">
              <TriggerRows
                triggers={triggers.triggers.filter((one) => one.workflow === undefined)}
                label="Triggers on every workflow"
                saved={answered}
                onOpen={(summary, level) => openTrigger({ kind: "open", summary, ...(level === undefined ? {} : { level }) })}
              />
              <div>
                <Button variant="secondary" onClick={() => openTrigger({ kind: "new", init: {} })}>
                  Add trigger
                </Button>
              </div>
            </div>
          </section>
        )}
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
          onPanel={openPanel}
          {...(triggers === undefined ? {} : { triggers, saved: answered, onTrigger: openTrigger })}
          onChange={change}
          onSave={() => void save()}
          onDiscuss={onDiscuss === undefined ? undefined : discuss}
        />
      )}
      {triggers === undefined || triggerEdit === null ? null : (
        <TriggerSheet
          key={triggerEdit.at}
          binding={triggers}
          target={triggerEdit.target}
          workflow={open === null || open.def.id === "" ? undefined : open.def.id}
          steps={open === null ? [] : open.def.steps.map((one) => one.id).filter((one) => one !== "")}
          onSaved={(saved) => setAnswered((was) => new Map(was).set(identityKey(saved), saved))}
          onRemoved={() => setTriggerEdit(null)}
          onClose={() => setTriggerEdit(null)}
        />
      )}
    </div>
  );
}

/** The mini graph's geometry, in its own units: a dot's radius, the gap between two, and the room round them. */
const DOT = 3;
const GAP = 14;
const PAD = 2;
const ARC = 5;

/**
 * The workflow as a row's small mark: a dot a step, in order, and a thin
 * arc over the dots from a step back to the earlier one it sends work to — the
 * canvas's own picture of it, drawn from the same definition.
 */
function MiniGraph({ def }: { def: Definition }) {
  const named = (step: Step, at: number) => (step.id === "" ? `step ${at + 1}` : step.id);
  const backs = def.steps.flatMap((step, at) => {
    const to = step.returnsTo === "" ? -1 : def.steps.findIndex((one) => one.id === step.returnsTo);
    return to !== -1 && to < at ? [{ from: at, to }] : [];
  });
  /** How high an arc stands over the dots: further for a longer way back, up to three rises. */
  const peakOf = (one: { from: number; to: number }) => ARC * Math.min(one.from - one.to, 3);
  const high = backs.length === 0 ? 0 : Math.max(...backs.map(peakOf));
  const width = PAD * 2 + DOT * 2 + (def.steps.length - 1) * GAP;
  const base = PAD + DOT + high;
  const said = `${def.steps.map(named).join(", ")}${backs
    .map((one) => `. ${named(def.steps[one.from]!, one.from)} returns to ${named(def.steps[one.to]!, one.to)}`)
    .join("")}`;
  return (
    <Tooltip asChild label={said}>
      <svg
        className="armada-wf-mini"
        role="img"
        aria-label={said}
        width={width}
        height={base + DOT + PAD}
        viewBox={`0 0 ${width} ${base + DOT + PAD}`}
      >
        {backs.map((one) => {
          const x1 = PAD + DOT + one.from * GAP;
          const x2 = PAD + DOT + one.to * GAP;
          const top = base - DOT - 2 * peakOf(one);
          return (
            <path key={`${one.from}-${one.to}`} className="armada-wf-mini__back" d={`M ${x1} ${base - DOT} Q ${(x1 + x2) / 2} ${top} ${x2} ${base - DOT}`} />
          );
        })}
        {def.steps.map((_, at) => (
          <circle key={at} className="armada-wf-mini__dot" style={{ fill: `var(${bandOf(def.steps[at]!, false).token})` }} cx={PAD + DOT + at * GAP} cy={base} r={DOT} />
        ))}
      </svg>
    </Tooltip>
  );
}

function Row({ entry, def, on, onOpen }: { entry: Resolved; def: Definition | undefined; on: boolean; onOpen: () => void }) {
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
        {leftOut || def === undefined ? null : <MiniGraph def={def} />}
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
  triggers,
  saved,
  onTrigger,
  onChange,
  onSave,
  onDiscuss,
}: {
  open: Open;
  repository: string;
  manifests: readonly ManifestOption[];
  entries: readonly Entry[];
  panel: Panel;
  onPanel: (panel: Panel) => void;
  triggers?: TriggersBinding;
  saved?: ReadonlyMap<string, TriggerSaved>;
  onTrigger?: (target: TriggerTarget) => void;
  onChange: (next: (def: Definition) => Definition) => void;
  onSave: () => void;
  onDiscuss: (() => void) | undefined;
}) {
  const { def, standing, refusals } = open;
  const ScopeGlyph = def.scope === KIT ? Briefcase : FolderGit2;
  const scopeName = (scope: string) => (scope === KIT ? "Kit" : (manifests.find((one) => one.id === scope)?.name ?? scope));
  const marked = useMemo(() => {
    const by = new Map<number, string[]>();
    for (const one of refusals) if (one.step !== undefined) by.set(one.step, [...(by.get(one.step) ?? []), one.why]);
    return by;
  }, [refusals]);
  // Pressing a refusal opens the panel it is fixed in and lands on the field.
  const [focus, setFocus] = useState<{ field: string; n: number } | null>(null);
  function fix(one: Refusal) {
    if (one.field === "Add step") {
      document.querySelector<HTMLElement>(".armada-wf-stage__acts button:nth-of-type(2)")?.focus();
      return;
    }
    onPanel(one.step === undefined ? "workflow" : one.step);
    setFocus({ field: one.field, n: (focus?.n ?? 0) + 1 });
  }
  useEffect(() => {
    if (focus === null) return;
    const land = window.setTimeout(() => {
      const label = [...document.querySelectorAll(".armada-wf-panel label")].find((one) => one.textContent === focus.field);
      const control =
        label === undefined ? null : label instanceof HTMLLabelElement && label.htmlFor !== "" ? document.getElementById(label.htmlFor) : label.querySelector("input");
      control?.focus();
    }, 120);
    return () => window.clearTimeout(land);
  }, [focus]);
  // The Triggers that fire at a step: this workflow's and every workflow's, and the ones that name the step or none.
  const listed = triggers?.triggers;
  const triggersAt = useCallback(
    (step: Step) => (listed === undefined ? [] : firingAt(listed, def.id, step)),
    [listed, def.id],
  );
  const { nodes, edges } = useMemo(() => graphOf(def, marked, panel, onPanel, triggersAt), [def, marked, panel, onPanel, triggersAt]);
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
        <Tooltip label={scopeName(def.scope)}>
          <span className="armada-wf-stage__mark" role="img" aria-label={scopeName(def.scope)}>
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
                {one.field === "" ? (
                  <span className="armada-wf-refusal">{one.why}</span>
                ) : (
                  <button type="button" className="armada-wf-refusal" onClick={() => fix(one)}>
                    <code>{`${one.step === undefined ? "Workflow" : stepName(def.steps[one.step], one.step)} · ${one.field}`}</code>
                    <span>{one.why}</span>
                  </button>
                )}
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
                scopeName={scopeName}
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
                {...(triggers === undefined || onTrigger === undefined
                  ? {}
                  : {
                      triggers: triggersAt(def.steps[panel]),
                      saved,
                      onOpenTrigger: (summary: TriggerSummary, level?: TriggerLevel) =>
                        onTrigger({ kind: "open", summary, ...(level === undefined ? {} : { level }) }),
                      onAddTrigger: (when: TriggerMoment) =>
                        onTrigger({
                          kind: "new",
                          init: { workflow: def.id, when, step: when === "pr_opened" ? "" : def.steps[panel]!.id },
                        }),
                    })}
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

/** The words a select offers, and the one the file holds where it is not among them, so opening a file never changes it. */
function withHeld(offered: readonly string[], held: string): string[] {
  return held === "" || offered.includes(held) ? [...offered] : [...offered, held];
}

function stepName(step: Step | undefined, at: number): string {
  return step === undefined || step.id === "" ? `Step ${at + 1}` : step.id;
}

function WorkflowFields({
  def,
  repository,
  manifests,
  scopeName,
  replaces,
  invalid,
  onChange,
}: {
  def: Definition;
  repository: string;
  manifests: readonly ManifestOption[];
  scopeName: (scope: string) => string;
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
          <option key={one.id} value={one.id}>
            {one.name}
          </option>
        ))}
      </Select>
      <span className="armada-wf-panel__fact">{fileOf(def, repository, scopeName)}</span>
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
  triggers,
  saved,
  onOpenTrigger,
  onAddTrigger,
  onChange,
  onRemove,
}: {
  step: Step;
  at: number;
  steps: readonly Step[];
  refused: boolean;
  /** The Triggers that fire at this step. Absent draws no section. */
  triggers?: readonly TriggerSummary[];
  saved?: ReadonlyMap<string, TriggerSaved> | undefined;
  onOpenTrigger?: (summary: TriggerSummary, level?: TriggerLevel) => void;
  onAddTrigger?: (when: TriggerMoment) => void;
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
      <Select label="Evidence" value={step.evidence} onChange={(event) => onChange({ evidence: event.target.value })}>
        <option value="" />
        {withHeld(EVIDENCE, step.evidence).map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
      <Select label="Check" value={step.check} onChange={(event) => onChange({ check: event.target.value })}>
        {withHeld(CHECKS, step.check).map((one) => (
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
          min={0}
          value={step.iterationCap}
          onChange={(event) => onChange({ iterationCap: event.target.valueAsNumber })}
        />
      )}
      {step.delivers !== true ? null : (
        <Switch
          checked={step.draftPr === true}
          // Off says nothing again where the file said nothing before, so a switch that was never set is not written as `ready`.
          onChange={(event) =>
            onChange({ draftPr: event.target.checked ? true : typeof step.carried?.draft_pr === "boolean" ? false : undefined })
          }
        >
          Draft PR
        </Switch>
      )}
      {triggers === undefined || onOpenTrigger === undefined || onAddTrigger === undefined || step.id === "" ? null : (
        <div className="armada-triggers__section">
          <TriggerRows triggers={triggers} label={`Triggers on ${step.id}`} saved={saved} onOpen={onOpenTrigger} />
          <span className="armada-triggers__acts">
            {step.delivers === true ? (
              <Button variant="secondary" onClick={() => onAddTrigger("pr_opened")}>
                Add trigger on PR opened
              </Button>
            ) : null}
            <Button variant="secondary" onClick={() => onAddTrigger("step_starts")}>
              Add trigger on start
            </Button>
            <Button variant="secondary" onClick={() => onAddTrigger("step_passes")}>
              Add trigger on pass
            </Button>
          </span>
        </div>
      )}
      <div>
        <Button variant="ghost" onClick={onRemove}>
          <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove step
        </Button>
      </div>
    </>
  );
}
