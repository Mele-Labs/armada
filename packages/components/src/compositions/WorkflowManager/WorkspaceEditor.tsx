import { ListPlus, Split } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { editorLanes, stepsOf } from "../LaneCanvas/build";
import { ruleBetween, uniqueId, withNewRule, withOverride, withReads, withRule, withStateAfter, withStateLabel, withStepAfter, withStepWhen, withStepsOrder, withoutRule, withoutState, withoutStep } from "../LaneCanvas/edit";
import type { RulePatch } from "../LaneCanvas/edit";
import { LaneCanvas } from "../LaneCanvas/LaneCanvas";
import type { LaneEditing, LanePick } from "../LaneCanvas/LaneCanvas";
import { lifecyclesFor, nodeId, whenSaid } from "../LaneCanvas/model";
import type { Autonomy, LifecycleDef, Rule, StateDef, Waits, Workspace } from "../LaneCanvas/model";
import { MOMENTS, endsOnOf, fireGroupsOf, momentOf, subjectsOf, waitsOf } from "../LaneCanvas/waits";
import type { WaitSubject } from "../LaneCanvas/waits";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { SentFromComposer } from "../SessionComposer/SessionComposer";
import type { ManagedStep, ManagedWorkflow, WorkflowActAnswer, WorkflowRef } from "./types";

/**
 * The Workspace a workflow is edited in: its steps on top of a lane for each lifecycle it uses, and rules as edges. **A mock's editor**: no Fleet serves a Workspace yet.
 * **Runs in** is a row of tabs over the canvas: All of the Workspace is the base, and each repository is the base with its own overrides, as a site builder edits a breakpoint. A repository's tab draws the
 * lanes as the workflow runs there, marks the nodes that differ there, and what is edited on it is an override for that repository alone. The panel shows the picked tab's one value, and says where a
 * repository's differs from the base. **Under the tabs is the box that tells an agent what to change** (`above`), always on show.
 *
 * **What is picked opens in the app's side panel**, over the canvas and as wide as the pull request's, drawn into the slot the host gives (`LanePanel`), since the panel is the host's and this is a component:
 * its settings as live fields, saved as they are changed, or a box to tell an agent. **Each row has its label once**, and the field in it is named by that label. A rule's When and a step's Ends when are two
 * dropdowns, the thing and then its moment or state. **There is no read only state and no mode to enter.** Hand editing is on the canvas too: a toolbar over what is picked, whose Edit goes to the fields, and a connector from
 * one node to another. **The first change on Armada's workflow makes the person's own copy, quietly**: the change lands on it, and nothing here says copy.
 */
export type WorkspaceEditing = {
  value: Workspace;
  /** The Workspace after a change by hand. Fleet would refuse it on a copy that is not the person's. */
  onChange: (next: Workspace) => void;
  /** The app's side panel, where what is picked is drawn. */
  panel: LanePanel;
  /** The workflow's own steps changed: written to its definition. */
  onSteps: (one: WorkflowRef, change: StepsChange) => void;
};

/** The app's side panel as the editor asks for it: opened for what is picked, and its body given back as a slot to draw into. */
export type LanePanel = {
  /** The panel is open on the editor's pick. */
  open: boolean;
  /** Where the pick is drawn, once the panel has drawn it. */
  slot: HTMLElement | null;
  show: (what: { title: string; heading: string; crumb: string }) => void;
  close: () => void;
};

/** A change to the workflow's own steps, which the host writes to the definition. */
export type StepsChange =
  | { kind: "label"; id: string; label: string }
  | { kind: "add"; after: string; id: string; label: string }
  | { kind: "remove"; id: string }
  /** The order the steps now run in. */
  | { kind: "order"; ids: readonly string[] };

const NONE: readonly never[] = [];

const NOBODY = () => undefined;

const AUTONOMY_SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

const refOf = (id: string): { lane: string; item: string } => ({ lane: id.slice(0, id.indexOf("/")), item: id.slice(id.indexOf("/") + 1) });
const splitValues = (text: string): string[] => text.split(",").map((one) => one.trim()).filter((one) => one !== "");

/** The fields a step's card takes when the panel draws it, each given the id of the row label that names it. */
export type StepRows = {
  name: (labelledBy: string) => ReactNode;
  ends: (labelledBy: string) => ReactNode;
  /** Absent where the step is passed over for nothing. */
  passedOver?: (labelledBy: string) => ReactNode;
};

/** What the panel holds for the pick: its fields, or a box to tell an agent about it. */
type Mode = "fields" | "tell";

export function WorkspaceEditor({
  editing,
  workflow,
  editable,
  proposed = NONE,
  stepCard,
  above,
  panel,
  onMakeCopy,
  onTell,
  onSteps,
}: {
  editing: WorkspaceEditing;
  workflow: ManagedWorkflow;
  /** The person's own copy. Armada's is not, and the first change on it makes one. */
  editable: boolean;
  /** The rules a change waiting on Approve would add, drawn as a diff. */
  proposed?: readonly Rule[];
  /** The step card for the step picked: its words, what ends it, what drifted. `rows` draws its name and what ends it as the fields of the panel, in the card's own rows. */
  stepCard: (step: ManagedStep, at: number, rows?: StepRows) => ReactNode;
  /** What stands under the tabs and over the canvas: the box that tells an agent what to change. */
  above?: ReactNode;
  panel: LanePanel;
  /** Makes the person's own copy of Armada's workflow, behind the first change made on it. */
  onMakeCopy: () => void;
  /** An agent told a change about what is picked. */
  onTell: (request: string) => Promise<WorkflowActAnswer>;
  onSteps: (change: StepsChange) => void;
}) {
  const workspace = editing.value;
  const [picked, setPicked] = useState<LanePick | undefined>();
  const [mode, setMode] = useState<Mode>("fields");
  // The tab: the Workspace's base, or one repository's.
  const [scope, setScope] = useState("workspace");
  const repository = workspace.repositories.find((one) => one.id === scope);
  useEffect(() => setScope("workspace"), [workflow.id]);
  // The steps are held while a copy's are read, so the canvas is not taken away and drawn again as the person's own copy opens.
  const read = useRef(workflow.steps);
  if (workflow.steps.length > 0) read.current = workflow.steps;
  const steps = workflow.steps.length > 0 ? workflow.steps : read.current;
  const drawn = useMemo(() => editorLanes({ workspace, workflow: workflow.id, steps, proposed, scope: repository?.id ?? "workspace" }), [workspace, workflow.id, steps, proposed, repository?.id]);

  // **The first change on Armada's workflow makes the person's own copy**, without a word: the change is made on the Workspace as it is, and the host makes the copy behind it.
  const change = (next: Workspace) => {
    editing.onChange(next);
    if (!editable) onMakeCopy();
  };
  const writeSteps = onSteps;
  const subjects = useMemo(() => subjectsOf(workspace), [workspace]);

  const names = useMemo(() => {
    const out = new Map<string, { label: string; lane: string }>();
    for (const lane of drawn.lanes) for (const node of lane.nodes) out.set(node.id, { label: node.label, lane: lane.title });
    return out;
  }, [drawn]);

  const nameOfPick = (at: LanePick): { title: string; heading: string; crumb: string } => {
    if (at.kind === "rule") {
      const rule = [...workspace.rules, ...proposed].find((one) => one.id === at.id);
      return { title: `Rule: ${rule?.label ?? at.id}`, heading: rule?.label ?? at.id, crumb: "Rule" };
    }
    const named = names.get(at.id);
    return { title: `${named?.lane ?? ""}: ${named?.label ?? at.id}`.replace(/^: /, ""), heading: named?.label ?? at.id, crumb: named?.lane ?? "" };
  };

  const pick = (next: LanePick | undefined, then: Mode = "fields") => {
    setPicked(next);
    setMode(then);
    if (next === undefined) panel.close();
    else panel.show(nameOfPick(next));
  };

  // The panel closed by its own close, or by something else opened in it, lets go of the pick.
  const was = useRef(false);
  useEffect(() => {
    if (was.current && !panel.open) {
      setPicked(undefined);
      setMode("fields");
    }
    was.current = panel.open;
  }, [panel.open]);
  // Leaving the workflow takes its panel with it.
  const held = useRef({ picked, close: panel.close });
  held.current = { picked, close: panel.close };
  useEffect(
    () => () => {
      if (held.current.picked !== undefined) held.current.close();
    },
    [],
  );

  // The panel's title follows a name changed in it.
  const named = picked === undefined ? undefined : nameOfPick(picked);
  useEffect(() => {
    if (picked !== undefined && panel.open && named !== undefined) panel.show(named);
    // Only a new name asks the panel again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [named?.title]);

  /** Edit goes to the first field of what is picked, in the panel, once the panel shows it. */
  const toFields = () => {
    setMode("fields");
    window.setTimeout(() => panel.slot?.querySelector<HTMLElement>("input:not([type='radio']), textarea, select")?.focus(), 0);
  };

  const addAfter = (at: LanePick) => {
    if (at.kind !== "node") return;
    const { lane, item } = refOf(at.id);
    if (lane === "workflow") {
      const id = uniqueId("New step", steps.map((one) => one.id));
      change(withStepAfter(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), item, id));
      writeSteps({ kind: "add", after: item, id, label: "New step" });
      pick({ kind: "node", id: nodeId("workflow", id) });
      return;
    }
    const made = withStateAfter(workspace, workflow.id, lane, item, "New state");
    change(made.workspace);
    pick({ kind: "node", id: nodeId(lane, made.id) });
  };

  const remove = (at: LanePick) => {
    if (at.kind === "rule") change(withoutRule(workspace, at.id));
    else {
      const { lane, item } = refOf(at.id);
      if (lane === "workflow") {
        change(withoutStep(workspace, workflow.id, item));
        writeSteps({ kind: "remove", id: item });
      } else change(withoutState(workspace, lane, item));
    }
    pick(undefined);
  };

  const connect = (from: string, to: string) => {
    if (from === to) return;
    const a = refOf(from);
    const b = refOf(to);
    if (a.lane === "workflow" && b.lane === "workflow") {
      // Between two steps it is order: the one it ends at comes right after the one it starts from.
      const ids = steps.map((one) => one.id).filter((one) => one !== b.item);
      ids.splice(ids.indexOf(a.item) + 1, 0, b.item);
      change(withStepsOrder(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), ids));
      writeSteps({ kind: "order", ids });
      pick({ kind: "node", id: to });
      return;
    }
    const rule = ruleBetween(workspace, workflow.id, a, b, { from: names.get(from)?.label ?? a.item, to: names.get(to)?.label ?? b.item, lane: names.get(to)?.lane ?? b.lane });
    change(withNewRule(workspace, rule));
    pick({ kind: "rule", id: rule.id });
  };

  const handles: LaneEditing = {
    ...(repository === undefined ? {} : { scoped: true }),
    onEdit: toFields,
    onTell: () => setMode("tell"),
    onAddAfter: addAfter,
    onDelete: remove,
    onConnect: connect,
  };

  const inspected = (() => {
    if (picked === undefined) return null;
    if (picked.kind === "rule") {
      const rule = [...workspace.rules, ...proposed].find((one) => one.id === picked.id);
      if (rule === undefined) return null;
      const added = proposed.includes(rule);
      return (
        <RuleFacts
          rule={rule}
          workflow={workflow.id}
          workspace={workspace}
          steps={steps}
          scope={repository}
          added={added}
          onChange={(patch) => change(withRule(workspace, rule.id, patch))}
          onOverride={(scope, autonomy) => change(withOverride(workspace, rule.id, scope, autonomy))}
        />
      );
    }
    const { lane, item } = refOf(picked.id);
    if (lane === "workflow") {
      const index = steps.findIndex((one) => one.id === item);
      const step = steps[index];
      if (step === undefined) return null;
      const placed = stepsOf(workspace, workflow.id, steps);
      const laid = placed.find((one) => one.id === step.id) ?? { id: step.id, label: step.label, col: index };
      const waits = (which: "ends" | "passedOver") => (labelledBy: string) => (
        <WaitsPair
          labelledBy={labelledBy}
          subjects={subjects}
          value={waitsOf(laid, which, which === "ends" ? step.ends : step.passedOver, subjects)}
          onChange={(next) => change(withStepWhen(workspace, workflow.id, placed, step.id, which, next, endsOnOf(next, subjects)))}
        />
      );
      return stepCard(step, index, {
        name: (labelledBy) => <Field labelledBy={labelledBy} value={step.label} onCommit={(label) => label !== "" && writeSteps({ kind: "label", id: step.id, label })} />,
        ends: waits("ends"),
        ...(step.passedOver === undefined && laid.passedOver === undefined ? {} : { passedOver: waits("passedOver") }),
      });
    }
    const lifecycle = workspace.lifecycles.find((one) => one.id === lane);
    const state = lifecycle?.states.find((one) => one.id === item);
    if (lifecycle === undefined || state === undefined) return null;
    return (
      <StateFacts
        lifecycle={lifecycle}
        state={state}
        workspace={workspace}
        scope={repository}
        onLabel={(label) => change(withStateLabel(workspace, lane, item, label))}
        onReads={(read) => change(withReads(workspace, lane, item, read))}
      />
    );
  })();

  const tell = async (sent: SentFromComposer): Promise<boolean> => {
    const text = sent.text.trim();
    if (text === "" || picked === undefined) return false;
    const where = nameOfPick(picked);
    const refused = await onTell(`${where.crumb}: ${where.heading}. ${text}`);
    if (refused !== undefined) return false;
    pick(undefined);
    return true;
  };

  const content = picked === undefined ? null : mode === "tell" ? <TellBox keep={`${workflow.id}/${picked.id}`} onSend={tell} /> : inspected;

  return (
    <section className="armada-wm-lanes" aria-label="Lanes">
      <div className="armada-wm-scope" role="group" aria-label="Runs in">
        <span className="armada-wm-scope__label" aria-hidden="true">
          Runs in
        </span>
        <div className="armada-wm-scope__tabs">
          <Tabs items={[{ id: "workspace", label: `All of ${workspace.name}` }, ...workspace.repositories.map((one) => ({ id: one.id, label: one.name }))]} value={repository?.id ?? "workspace"} onChange={setScope} />
        </div>
        {repository === undefined ? null : <p className="armada-wm-scope__says">{`Edits here change ${repository.name} only`}</p>}
      </div>
      {above}
      {/* Drawn once the steps are read: lanes that fill in after they open would pan to what arrived. */}
      {steps.length === 0 ? null : <LaneCanvas label={`Lanes of ${workflow.id}`} lanes={drawn.lanes} rules={drawn.rules} picked={picked} onPick={(next) => pick(next)} editing={handles} coveredBy={panel.open ? panel.slot : null} />}
      {content === null || panel.slot === null ? null : createPortal(<div className="armada-wm-inspector">{content}</div>, panel.slot)}
    </section>
  );
}

/** The box that tells an agent what to change about what is picked, answered with the diff the page draws. */
function TellBox({ keep, onSend }: { keep: string; onSend: (sent: SentFromComposer) => Promise<boolean> }) {
  return (
    <div className="armada-wm-tell">
      <SessionComposer
        working={false}
        bare
        label="Tell an agent what to change"
        send="secondary"
        sendLabel="Send to agent"
        model={null}
        effort={null}
        models={NONE}
        efforts={NONE}
        onTune={NOBODY}
        commands={NONE}
        taggable={NONE}
        tags={NONE}
        onTags={NOBODY}
        drawn={NONE}
        onDraw={NOBODY}
        onRemoveDrawn={NOBODY}
        draftKey={`workflows:tell:${keep}`}
        onSend={onSend}
      />
    </div>
  );
}

/** A field that edits a stored value and says it where it is left, or on Enter. Its name is its row's label: `labelledBy` is that label's id. */
function Field({ labelledBy, value, onCommit, mono = false }: { labelledBy: string; value: string; onCommit: (next: string) => void; mono?: boolean }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const next = text.trim();
    if (next !== value) onCommit(next);
  };
  return (
    <Input
      aria-labelledby={labelledBy}
      mono={mono}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
      }}
    />
  );
}

/** Words that run to several lines, shown whole: the box grows with them. Said where it is left, or on Enter; Shift and Enter starts a line. */
function Wrapped({ labelledBy, keep, value, onCommit }: { labelledBy: string; keep: string; value: string; onCommit: (next: string) => void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const next = text.trim();
    if (next !== value) onCommit(next);
  };
  return (
    <Textarea
      aria-labelledby={labelledBy}
      grow
      rows={1}
      keep={keep}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key !== "Enter" || event.shiftKey) return;
        event.preventDefault();
        commit();
      }}
    />
  );
}

/** Lines of text that edit a stored list, said where the box is left. `keep` names what the lines are about, so a line half typed is there when the panel comes back. */
function Lines({ labelledBy, keep, lines, onCommit }: { labelledBy: string; keep: string; lines: readonly string[]; onCommit: (next: string[]) => void }) {
  const [text, setText] = useState(lines.join("\n"));
  useEffect(() => setText(lines.join("\n")), [lines]);
  return (
    <Textarea
      aria-labelledby={labelledBy}
      rows={Math.max(2, lines.length)}
      keep={keep}
      value={text}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => {
        const next = text.split("\n").map((one) => one.trim()).filter((one) => one !== "");
        if (next.join("\n") !== lines.join("\n")) onCommit(next);
      }}
    />
  );
}

type Choice = { value: string; label: string };

/**
 * Two dropdowns on one row that set one fact, a thing and then what it does or is: `[ Build ] [ ends ]`. Both are named by the row's label, the second with a word that tells it from the first, so
 * the row says its name once. Each saves as it is changed. A value the lists do not hold opens on an empty choice.
 */
function Pair({
  labelledBy,
  first,
  second,
}: {
  labelledBy: string;
  first: { value: string; groups: readonly { label?: string; items: readonly Choice[] }[]; onChange: (value: string) => void };
  second: { value: string; hidden: string; items: readonly Choice[]; onChange: (value: string) => void };
}) {
  const hiddenId = useId();
  const empty = (value: string, items: readonly Choice[]) => value === "" || !items.some((one) => one.value === value);
  const all = first.groups.flatMap((group) => group.items);
  return (
    <div className="armada-wm-pair">
      <span id={hiddenId} className="armada-wm-hidden">
        {second.hidden}
      </span>
      <Select aria-labelledby={labelledBy} value={empty(first.value, all) ? "" : first.value} onChange={(event) => first.onChange(event.target.value)}>
        {empty(first.value, all) ? <option value="" /> : null}
        {first.groups.map((group) =>
          group.label === undefined ? (
            group.items.map((one) => (
              <option key={one.value} value={one.value}>
                {one.label}
              </option>
            ))
          ) : (
            <optgroup key={group.label} label={group.label}>
              {group.items.map((one) => (
                <option key={one.value} value={one.value}>
                  {one.label}
                </option>
              ))}
            </optgroup>
          ),
        )}
      </Select>
      <Select aria-labelledby={`${labelledBy} ${hiddenId}`} value={empty(second.value, second.items) ? "" : second.value} disabled={second.items.length === 0} onChange={(event) => second.onChange(event.target.value)}>
        {empty(second.value, second.items) ? <option value="" /> : null}
        {second.items.map((one) => (
          <option key={one.value} value={one.value}>
            {one.label}
          </option>
        ))}
      </Select>
    </div>
  );
}

/** What a step is ended by, or passed over for: the thing, then its state. Choosing another thing takes its first state. */
function WaitsPair({ labelledBy, subjects, value, onChange }: { labelledBy: string; subjects: readonly WaitSubject[]; value: Waits | undefined; onChange: (next: Waits) => void }) {
  const subject = subjects.find((one) => one.id === value?.on);
  return (
    <Pair
      labelledBy={labelledBy}
      first={{
        value: value?.on ?? "",
        groups: [{ items: subjects.map((one) => ({ value: one.id, label: one.label })) }],
        onChange: (on) => {
          const next = subjects.find((one) => one.id === on);
          const state = next?.states[0];
          if (next !== undefined && state !== undefined) onChange({ on: next.id, state: state.id });
        },
      }}
      second={{
        value: value?.state ?? "",
        hidden: "state",
        items: (subject?.states ?? []).map((one) => ({ value: one.id, label: one.label })),
        onChange: (state) => value !== undefined && onChange({ on: value.on, state }),
      }}
    />
  );
}

/** What differs in a repository: the glyph, named for the repository, beside what it says. */
function Differs({ repository, children }: { repository: string; children: ReactNode }) {
  return (
    <span className="armada-wm-inspector__differs">
      <Tooltip label={`Differs in ${repository}`}>
        <span role="img" aria-label={`Differs in ${repository}`}>
          <Split size={14} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      {children}
    </span>
  );
}

/** What a repository's tab says of a value it overrides: where it differs from the base, and the way back to the base's. */
function Changed({ repository, base, onUseBase }: { repository: string; base: string; onUseBase: () => void }) {
  return (
    <dd className="armada-wm-inspector__changed">
      <Differs repository={repository}>{`Changed for ${repository}`}</Differs>
      <Button type="button" variant="ghost" size="sm" onClick={onUseBase}>
        {`Use ${base}'s`}
      </Button>
    </dd>
  );
}

/**
 * A state: where Fleet reads it from, and the values that count as being in it, **for the tab that is picked**. On the base they are the Workspace's; on a repository's tab they are that
 * repository's, which are the base's until it overrides them, and an override says so and can be dropped. Edited on a tab, they change that tab's alone. **Each row has its label once**,
 * and the field in it is named by that label.
 */
function StateFacts({
  lifecycle,
  state,
  workspace,
  scope,
  onLabel,
  onReads,
}: {
  lifecycle: LifecycleDef;
  state: StateDef;
  workspace: Workspace;
  /** The repository whose tab is picked; absent on the base. */
  scope: { id: string; name: string } | undefined;
  onLabel: (label: string) => void;
  onReads: (read: { source: string; scope: string; values: readonly string[] }) => void;
}) {
  const id = useId();
  const sources = [...new Set(state.reads.map((one) => one.source))];
  const valuesOf = (source: string, at: string) => state.reads.find((one) => one.source === source && one.scope === at)?.values;
  return (
    <div className="armada-wm-step">
      <dl className="armada-wm-step__facts">
        {scope === undefined ? (
          <>
            <dt id={`${id}-name`}>Name</dt>
            <dd>
              <Field labelledBy={`${id}-name`} value={state.label} onCommit={(label) => label !== "" && onLabel(label)} />
            </dd>
          </>
        ) : null}
        {sources.length === 1 && sources[0] === lifecycle.from ? null : (
          <>
            <dt>Read from</dt>
            <dd>{lifecycle.from}</dd>
          </>
        )}
        {sources.flatMap((source, at) => {
          const base = valuesOf(source, "workspace") ?? [];
          const own = scope === undefined ? undefined : valuesOf(source, scope.id);
          const shown = own ?? base;
          return [
            <dt key={`${source}:name`} id={`${id}-${at}`}>{`${source} statuses`}</dt>,
            <dd key={source}>
              <Wrapped
                labelledBy={`${id}-${at}`}
                keep={`workflows:statuses:${lifecycle.id}/${state.id}/${source}/${scope?.id ?? "workspace"}`}
                value={shown.join(", ")}
                onCommit={(text) => {
                  const values = splitValues(text);
                  // On a repository's tab the base's own words are no override.
                  onReads({ source, scope: scope?.id ?? "workspace", values: scope !== undefined && values.join(", ") === base.join(", ") ? [] : values });
                }}
              />
            </dd>,
            scope === undefined || own === undefined ? null : <Changed key={`${source}:changed`} repository={scope.name} base={workspace.name} onUseBase={() => onReads({ source, scope: scope.id, values: [] })} />,
          ];
        })}
      </dl>
    </div>
  );
}

/**
 * A rule: when it fires, what it checks, what it does and how far it may go alone, **for the tab that is picked**. On a repository's tab the autonomy is that repository's, which is the base's
 * until it is overridden, and an override says so and can be dropped. Its words and its place are the base's, edited there. **When is two dropdowns**, the step or state and then the moment, so
 * it is changed in two presses. Each row has its label once.
 */
function RuleFacts({
  rule,
  workflow,
  workspace,
  steps,
  scope,
  added,
  onChange,
  onOverride,
}: {
  rule: Rule;
  workflow: string;
  workspace: Workspace;
  steps: readonly { id: string; label: string }[];
  /** The repository whose tab is picked; absent on the base. */
  scope: { id: string; name: string } | undefined;
  /** A rule the proposed change would add is not the Workspace's yet: it is shown, and Approve makes it. */
  added: boolean;
  onChange: (patch: RulePatch) => void;
  onOverride: (scope: string, autonomy: Autonomy) => void;
}) {
  const id = useId();
  const own = scope === undefined ? undefined : rule.overrides?.find((one) => one.scope === scope.id)?.autonomy;
  const autonomy = own ?? rule.autonomy;
  // Its name, its place and its lines are the base's: they are edited there, not per repository.
  const writing = scope === undefined && !added;
  const kind = rule.does[0]?.act ?? "agent";
  const lifecycles = lifecyclesFor(workspace, workflow);
  const groups = fireGroupsOf(steps, lifecycles);
  const from = nodeId(rule.from.lane, rule.from.item);
  // A node the workflow does not draw still names its place, so the choice shows what the rule has.
  const held: readonly { label?: string; items: readonly Choice[] }[] = groups.some((group) => group.items.some((one) => one.value === from)) ? groups : [{ items: [{ value: from, label: whenSaid({ ...rule, at: undefined }, lifecycles, steps) }] }, ...groups];
  const moments = rule.from.lane === "workflow" ? MOMENTS.step : MOMENTS.state;
  return (
    <div className="armada-wm-step" data-added={added || undefined}>
      {added ? (
        <div className="armada-wm-step__band">
          <span className="armada-wm-step__marks">
            <Tooltip label="Added by the proposed change">
              <span role="img" aria-label="Added by the proposed change">
                <ListPlus size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          </span>
        </div>
      ) : null}
      <dl className="armada-wm-step__facts">
        {writing ? (
          <>
            <dt id={`${id}-name`}>Name</dt>
            <dd>
              <Field labelledBy={`${id}-name`} value={rule.label} onCommit={(label) => label !== "" && onChange({ label })} />
            </dd>
          </>
        ) : null}
        <dt id={`${id}-when`}>When</dt>
        <dd>
          {writing ? (
            <Pair
              labelledBy={`${id}-when`}
              first={{
                value: from,
                groups: held,
                onChange: (value) => {
                  const ref = refOf(value);
                  onChange({ from: ref, at: ref.lane === "workflow" ? "starts" : "enters" });
                },
              }}
              second={{ value: momentOf(rule), hidden: "moment", items: moments, onChange: (at) => onChange({ at: at as NonNullable<Rule["at"]> }) }}
            />
          ) : (
            whenSaid(rule, workspace.lifecycles, steps)
          )}
        </dd>
        {writing || rule.ifs.length > 0 ? (
          <>
            <dt id={`${id}-if`}>If</dt>
            <dd>
              {writing ? (
                <Lines labelledBy={`${id}-if`} keep={`workflows:if:${rule.id}`} lines={rule.ifs} onCommit={(ifs) => onChange({ ifs })} />
              ) : (
                <ul className="armada-wm-step__ends" aria-labelledby={`${id}-if`}>
                  {rule.ifs.map((one) => (
                    <li key={one}>{one}</li>
                  ))}
                </ul>
              )}
            </dd>
          </>
        ) : null}
        <dt id={`${id}-do`}>Do</dt>
        <dd>
          {writing ? (
            <Lines labelledBy={`${id}-do`} keep={`workflows:do:${rule.id}`} lines={rule.does.map((one) => one.text)} onCommit={(lines) => onChange({ does: lines.map((text, at) => ({ act: rule.does[at]?.act ?? kind, text })) })} />
          ) : (
            <ul className="armada-wm-step__ends" aria-labelledby={`${id}-do`}>
              {rule.does.map((one) => (
                <li key={one.text}>{one.text}</li>
              ))}
            </ul>
          )}
        </dd>
        <dt id={`${id}-autonomy`}>Autonomy</dt>
        <dd>
          <RadioGroup labelledBy={`${id}-autonomy`}>
            {(["acts", "asks", "off"] as const).map((one) => (
              <Radio key={one} name={`autonomy:${rule.id}`} checked={autonomy === one} disabled={added} onChange={() => (scope === undefined ? onChange({ autonomy: one }) : onOverride(scope.id, one))}>
                {AUTONOMY_SAID[one]}
              </Radio>
            ))}
          </RadioGroup>
        </dd>
        {scope === undefined || own === undefined ? null : <Changed repository={scope.name} base={workspace.name} onUseBase={() => onOverride(scope.id, rule.autonomy)} />}
      </dl>
    </div>
  );
}

/** One rule a proposed change adds, drawn as the step cards of a proposal are: its place and name, then its facts. */
export function RuleChangeCard({ rule, workspace, steps }: { rule: Rule; workspace: Workspace; steps: readonly { id: string; label: string }[] }) {
  return (
    <div role="group" aria-label={rule.label} className="armada-wm-diff__card" data-kind="added">
      <div className="armada-wm-diff__card-band">
        <span className="armada-wm-step__label">{rule.label}</span>
        <span className="armada-wm-diff__card-sign">
          <Tooltip label="Added">
            <span role="img" aria-label="Added">
              <ListPlus size={14} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        </span>
      </div>
      <dl className="armada-wm-step__facts">
        <dt>When</dt>
        <dd>{whenSaid(rule, workspace.lifecycles, steps)}</dd>
        {rule.ifs.length === 0 ? null : (
          <>
            <dt>If</dt>
            <dd>
              <ul className="armada-wm-step__ends">
                {rule.ifs.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </dd>
          </>
        )}
        <dt>Do</dt>
        <dd>
          <ul className="armada-wm-step__ends">
            {rule.does.map((one) => (
              <li key={one.text}>{one.text}</li>
            ))}
          </ul>
        </dd>
        <dt>Autonomy</dt>
        <dd>{AUTONOMY_SAID[rule.autonomy]}</dd>
      </dl>
    </div>
  );
}
