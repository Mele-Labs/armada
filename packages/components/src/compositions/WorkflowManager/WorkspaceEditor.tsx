import { Clock, FolderPlus, ListPlus, Pencil, Plus, SlidersHorizontal, Split, Trash2 } from "lucide-react";
import { useEffect, useId, useMemo, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { createPortal } from "react-dom";
import type { LifecycleRule } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Popover } from "../../primitives/Popover/Popover";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tabs } from "../../primitives/Tabs/Tabs";
import type { TabsItem } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { changedAt, editorLanes, stepsOf } from "../LaneCanvas/build";
import {
  ruleBetween,
  uniqueId,
  withNewRule,
  withReads,
  withRuleAt,
  withRuleWireAt,
  withAddedStep,
  withLifecycleAdded,
  withLifecycleLabel,
  withoutLifecycle,
  withStateAfter,
  withStateLabelAt,
  withStepAddedAt,
  withStepAfter,
  withStepLayer,
  withStepWhen,
  withStepsOrder,
  withoutRule,
  withoutRuleOverride,
  withoutState,
  withoutStateOverride,
  withoutStep,
  withoutStepLayer,
} from "../LaneCanvas/edit";
import type { RuleFields, StepFields } from "../LaneCanvas/edit";
import { LaneCanvas } from "../LaneCanvas/LaneCanvas";
import type { LaneEditing, LanePick } from "../LaneCanvas/LaneCanvas";
import { DEFAULT, END_ITEM, REMOVAL_WAITS, START_ITEM, existsAt, layersAt, levelsOf, lifecyclesAt, nodeId, readsAt, ruleAt, stateDiffers, stateExistsAt, stateLabelAt, stepDiffers, stepsAt, whenSaid } from "../LaneCanvas/model";
import type { Autonomy, LifecycleDef, Ref, Rule, StateDef, Waits, Workspace } from "../LaneCanvas/model";
import { MOMENTS, endsOnOf, fireGroupsOf, momentOf, subjectsOf, waitsOf } from "../LaneCanvas/waits";
import type { WaitSubject } from "../LaneCanvas/waits";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { SentFromComposer } from "../SessionComposer/SessionComposer";
import { AutonomyPanel, autonomyLevels, autonomyRows } from "./Autonomy";
import type { AutonomyEditing } from "./Autonomy";
import { Field, Lines, Pair, Wrapped } from "./InspectorFields";
import type { Choice } from "./InspectorFields";
import { RuleForm } from "./RuleForm";
import type { RuleFormLifecycle } from "./RuleForm";
import type { ManagedStep, ManagedWorkflow, WorkflowActAnswer, WorkflowRef } from "./types";
import { WorkspaceForm } from "./WorkspaceForm";
import type { WorkspacesEditing } from "./WorkspaceForm";

/**
 * The Workspace a workflow is edited in: its steps on top of a lane for each lifecycle it uses, and rules as edges. **The host keeps it**: from the pieces Fleet serves, or from a scenario's mock.
 * **Runs in** is a row of tabs over the canvas, one for each level a value can be set at, as a site builder edits a breakpoint: **Default**, the workflow as it runs anywhere no Workspace or
 * repository changed it; All of the Workspace, the default with the Workspace's changes; and each repository, the Workspace's with its own. A tab draws the lanes as the workflow runs there, marks the nodes
 * that differ from it down, and what is edited on it is an override for that level alone, so a tab says in a line what an edit there changes. A tab that differs from the level above carries the changed
 * glyph. The panel shows the picked tab's one value, and says where it differs from the level above. **Under the tabs is the box that tells an agent what to change** (`above`), always on show.
 *
 * **What is picked opens in the app's side panel**, over the canvas and as wide as the pull request's, drawn into the slot the host gives (`LanePanel`), since the panel is the host's and this is a component:
 * its settings as live fields, saved as they are changed, or a box to tell an agent. **Each row has its label once**, and the field in it is named by that label. A rule's When and a step's Ends when are two
 * dropdowns, the thing and then its moment or state. **There is no mode to enter**: every field of every rule, state and step is live on every tab and every workflow, a rule a change
 * waiting on Approve would add excepted, and every one where the host keeps nothing a change could be saved to (`readOnly`). Hand editing is on the canvas too: a toolbar over what is picked, whose Edit goes to the fields, and a connector from one node to another. Adding and connecting work on every tab,
 * and what a tab adds is that level's own: it runs there and under it, and the tab can delete it. What a level above holds is skipped with a step's Passed over when. **The first change on Armada's workflow makes the person's own copy, quietly**: the change lands on it, and nothing here says copy.
 */
export type WorkspaceEditing = {
  value: Workspace;
  /** The Workspace after a change by hand. Fleet would refuse it on a copy that is not the person's. */
  onChange: (next: Workspace) => void;
  /** The app's side panel, where what is picked is drawn. */
  panel: LanePanel;
  /** The workflow's own steps changed: written to its definition. */
  onSteps: (one: WorkflowRef, change: StepsChange) => void;
  /** The lifecycles and rules are pieces the host keeps apart from the workflow: a change to them makes no copy of an Armada workflow, and a change to its steps does. Only a step can be told to an agent, which drafts changes to steps. */
  pieces?: true;
  /** Nothing a change could be saved to: the lanes are drawn, and no field or handle changes them. */
  readOnly?: true;
  /** Where the picked rule or lifecycle is kept, drawn under its fields: the acts on it, for the tab it is picked on. Absent, nothing stands there. */
  pieceActs?: (pick: { kind: "rule" | "lifecycle"; id: string }, tab: string) => ReactNode;
  /** Whether a rule, as it stands, lands on a node of the lanes, which is how it is drawn and picked. Absent, every rule does. */
  lands?: (rule: LifecycleRule) => boolean;
  /** Whether a connector from one node to another makes a rule Fleet can run. A pair that does not is offered no landing. Absent, any pair does. */
  canConnect?: (from: Ref, to: Ref) => boolean;
  /** The rules and lifecycles whose removal waits on a pull request, as `rule:<id>` and `lifecycle:<id>`. */
  pending?: ReadonlySet<string>;
  /** Making, editing and removing a Workspace: a button in the Runs in row, and a pencil on the Workspace's tab. Absent, neither is drawn. */
  workspaces?: WorkspacesEditing;
  /** The autonomy defaults for the picked tab, opened in the side panel from a button in the Runs in row. Absent, no button is drawn. */
  autonomy?: AutonomyEditing;
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

/** Whether a change touched the workflow's own steps, which a copy of the workflow holds, and not only the lifecycles and rules, which are pieces. */
const stepsDiffer = (a: Workspace, b: Workspace): boolean => {
  const steps = (workspace: Workspace) => JSON.stringify(Object.entries(workspace.workflows).map(([id, place]) => [id, place.steps, place.layers, place.added]));
  return steps(a) !== steps(b);
};

const NOBODY = () => undefined;

const AUTONOMY_SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

const refOf = (id: string): { lane: string; item: string } => ({ lane: id.slice(0, id.indexOf("/")), item: id.slice(id.indexOf("/") + 1) });
const splitValues = (text: string): string[] => text.split(",").map((one) => one.trim()).filter((one) => one !== "");

/** The fields a step's card takes when the panel draws it, each given the id of the row label that names it. */
export type StepRows = {
  /** Absent where the step's name is not this level's to change. */
  name?: (labelledBy: string) => ReactNode;
  /** Absent where nothing keeps what ends it: the card says it as the workflow's definition does. */
  ends?: (labelledBy: string) => ReactNode;
  /** The words are read and not edited here, where nothing keeps a level's own. */
  fixed?: true;
  /** Absent where the step is passed over for nothing. */
  passedOver?: (labelledBy: string) => ReactNode;
  /** Saves the words where a level below the default holds them. Absent, the words are the workflow's and the host saves them. */
  save?: (words: string) => Promise<boolean>;
  /** What the words box is remembered by, so each level keeps its own half typed words. */
  keep?: string;
  /** What stands under the rows: where the step differs from the level above, and the way back. */
  after?: ReactNode;
};

/** A tab the editor can stand on below the default, and what it says: the id the lanes are drawn for, its name in "Changed for", and the way back to the level above. */
type Level = { id: string; name: string; back: string };

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
  // What the side panel holds beside a pick: the autonomy defaults of the tab, or a Workspace's form where the form is drawn there.
  const [autonomyOpen, setAutonomyOpen] = useState(false);
  const [form, setForm] = useState<"new" | "edit" | undefined>();
  // The tab: the default's, the Workspace's, or one repository's. The page opens on the default.
  const [scope, setScope] = useState(DEFAULT);
  const repository = workspace.repositories.find((one) => one.id === scope);
  /** The level a tab below the default sets: absent on the default, which has no level above it. */
  const level: Level | undefined = scope === DEFAULT ? undefined : scope === "workspace" ? { id: scope, name: workspace.name, back: "Use the default" } : { id: scope, name: repository?.name ?? scope, back: `Use ${workspace.name}'s` };
  useEffect(() => setScope(DEFAULT), [workflow.id]);
  // The steps are held while a copy's are read, so the canvas is not taken away and drawn again as the person's own copy opens.
  const read = useRef(workflow.steps);
  if (workflow.steps.length > 0) read.current = workflow.steps;
  const defined = workflow.steps.length > 0 ? workflow.steps : read.current;
  const place = workspace.workflows[workflow.id];
  /** The steps as they run on this tab: the definition's, and the ones added at this level or above. */
  const steps = useMemo(() => stepsAt(defined, place, scope, (one): ManagedStep => ({ id: one.id, label: one.label, ends: "", words: one.words ?? "" })), [defined, place, scope]);
  const drawn = useMemo(() => editorLanes({ workspace, workflow: workflow.id, steps, proposed, scope, ...(editing.pending === undefined ? {} : { pending: editing.pending }) }), [workspace, workflow.id, steps, proposed, scope, editing.pending]);

  // **The first change on Armada's workflow makes the person's own copy**, without a word: the change is made on the Workspace as it is, and the host makes the copy behind it.
  const change = (next: Workspace) => {
    if (editing.readOnly === true) return;
    editing.onChange(next);
    if (!editable && (editing.pieces !== true || stepsDiffer(workspace, next))) onMakeCopy();
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
      const found = [...workspace.rules, ...proposed].find((one) => one.id === at.id);
      const rule = found === undefined ? undefined : ruleAt(found, scope);
      return { title: `Rule: ${rule?.label ?? at.id}`, heading: rule?.label ?? at.id, crumb: "Rule" };
    }
    if (at.kind === "lane") {
      const label = workspace.lifecycles.find((one) => one.id === at.id)?.label ?? at.id;
      return { title: label, heading: label, crumb: "Lifecycle" };
    }
    const named = names.get(at.id);
    return { title: `${named?.lane ?? ""}: ${named?.label ?? at.id}`.replace(/^: /, ""), heading: named?.label ?? at.id, crumb: named?.lane ?? "" };
  };

  const pick = (next: LanePick | undefined, then: Mode = "fields") => {
    setPicked(next);
    setMode(then);
    setAutonomyOpen(false);
    if (editing.workspaces?.take === "panel") setForm(undefined);
    if (next === undefined) panel.close();
    else panel.show(nameOfPick(next));
  };

  // The panel closed by its own close, or by something else opened in it, lets go of the pick.
  const was = useRef(false);
  useEffect(() => {
    if (was.current && !panel.open) {
      setPicked(undefined);
      setMode("fields");
      setAutonomyOpen(false);
      if (editing.workspaces?.take === "panel") setForm(undefined);
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
    window.setTimeout(() => panel.slot?.querySelector<HTMLElement>("input:not([type='radio']), textarea, select")?.focus({ preventScroll: true }), 0);
  };

  const addAfter = (at: LanePick) => {
    if (at.kind !== "node") return;
    const { lane, item } = refOf(at.id);
    if (lane === "workflow") {
      const id = uniqueId("New step", [...defined.map((one) => one.id), ...(place?.added ?? []).map((one) => one.id)]);
      // On the default a step joins the workflow itself. Below it the step is that level's own: it runs there and under it, and the definition does not hold it.
      if (level === undefined) {
        change(withStepAfter(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), item, id));
        writeSteps({ kind: "add", after: item, id, label: "New step" });
      } else change(withStepAddedAt(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), item, id, "New step", scope));
      pick({ kind: "node", id: nodeId("workflow", id) });
      return;
    }
    const made = withStateAfter(workspace, workflow.id, lane, item, "New state", scope);
    change(made.workspace);
    pick({ kind: "node", id: nodeId(lane, made.id) });
  };

  const remove = (at: LanePick) => {
    if (at.kind === "rule") change(withoutRule(workspace, at.id));
    else if (at.kind === "lane") change(withoutLifecycle(workspace, at.id));
    else {
      const { lane, item } = refOf(at.id);
      if (lane === "workflow") {
        change(withoutStep(workspace, workflow.id, item));
        if (!(place?.added ?? []).some((one) => one.id === item)) writeSteps({ kind: "remove", id: item });
      } else change(withoutState(workspace, lane, item));
    }
    pick(undefined);
  };

  const connect = (from: string, to: string) => {
    if (from === to) return;
    const a = refOf(from);
    const b = refOf(to);
    if (a.lane === "workflow" && b.lane === "workflow") {
      // Between two steps it is order, which is the workflow's own: the one it ends at comes right after the one it starts from.
      if (level !== undefined) return;
      const ids = steps.map((one) => one.id).filter((one) => one !== b.item);
      ids.splice(ids.indexOf(a.item) + 1, 0, b.item);
      change(withStepsOrder(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), ids));
      writeSteps({ kind: "order", ids });
      pick({ kind: "node", id: to });
      return;
    }
    const rule = ruleBetween(workspace, workflow.id, a, b, { from: names.get(from)?.label ?? a.item, to: names.get(to)?.label ?? b.item, lane: names.get(to)?.lane ?? b.lane }, scope);
    change(withNewRule(workspace, rule));
    pick({ kind: "rule", id: rule.id });
  };

  /** A tab deletes what it added, and the default what the workflow holds. */
  const canDelete = (at: LanePick): boolean => {
    if (at.kind === "rule") return (workspace.rules.find((one) => one.id === at.id)?.scope ?? DEFAULT) === scope;
    if (at.kind === "lane") return lifecycleDeletable(at.id);
    const { lane, item } = refOf(at.id);
    if (lane === "workflow") return ((place?.added ?? []).find((one) => one.id === item)?.scope ?? DEFAULT) === scope;
    return (workspace.lifecycles.find((one) => one.id === lane)?.states.find((one) => one.id === item)?.scope ?? DEFAULT) === scope;
  };

  const pieces = editing.pieces === true;
  /** A lifecycle is deleted on the tab that added it, when a file of the person's or the repository's defines it and no rule names it: Armada's four are not files to delete. */
  const lifecycleDeletable = (id: string): boolean => {
    const found = workspace.lifecycles.find((one) => one.id === id);
    if (found === undefined || (found.scope ?? DEFAULT) !== scope) return false;
    if (pieces && (found.kept === undefined || found.kept === "armada")) return false;
    const named = `"lifecycle":"${id}"`;
    return !workspace.rules.some((rule) => rule.from.lane === id || rule.to.lane === id || [rule.wire, ...(rule.overrides ?? []).map((one) => one.wire)].some((wire) => wire !== undefined && JSON.stringify(wire).includes(named)));
  };
  /** Where the lifecycles and rules are pieces, a node is added only where something can keep it: the workflow's steps on the default, and the states of a lifecycle the tracker reads. */
  const canAdd = (at: LanePick): boolean => {
    if (!pieces || at.kind !== "node") return true;
    const { lane } = refOf(at.id);
    if (lane === "workflow") return level === undefined;
    const kind = workspace.lifecycles.find((one) => one.id === lane)?.kind;
    return kind === undefined || kind === "tracker";
  };
  const canConnect = (from: string, to: string): boolean => {
    const a = refOf(from);
    const b = refOf(to);
    if (from === to || [a.item, b.item].some((one) => one === START_ITEM || one === END_ITEM)) return false;
    // Between two steps it is order, which is the workflow's own and is set on the default.
    if (a.lane === "workflow" && b.lane === "workflow") return !pieces || level === undefined;
    return editing.canConnect?.(a, b) ?? true;
  };

  const handles: LaneEditing = {
    canDelete,
    canAdd,
    canConnect,
    // An agent drafts a change to the workflow's steps, so where the lifecycles and rules are pieces kept apart from it, only a step can be told.
    ...(editing.pieces === true ? { canTell: (at: LanePick) => at.kind === "node" && refOf(at.id).lane === "workflow" } : {}),
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
      const wire = ruleAt(rule, scope).wire;
      // Where rules are pieces, a rule is its piece: pickers over what Fleet can run, each change a piece kept whole at this tab.
      if (pieces && wire !== undefined && !added) {
        const here = lifecyclesOf(workspace, scope);
        return (
          <div className="armada-wm-step">
            {editing.pending?.has(`rule:${rule.id}`) === true ? <Pending /> : null}
            <dl className="armada-wm-step__facts">
              <RuleForm
                key={`${rule.id}/${scope}`}
                rule={wire}
                lifecycles={here}
                steps={stepsOf(workspace, workflow.id, steps, scope)}
                workflows={Object.keys(workspace.workflows)}
                lands={editing.lands ?? (() => true)}
                onChange={(next) => change(withRuleWireAt(workspace, rule.id, scope, next))}
              />
              {level === undefined || !(rule.overrides ?? []).some((one) => one.scope === scope) ? null : <Changed name={level.name} back={level.back} onUseAbove={() => change(withoutRuleOverride(workspace, rule.id, scope))} />}
            </dl>
          </div>
        );
      }
      return (
        <RuleFacts
          rule={rule}
          workflow={workflow.id}
          workspace={workspace}
          steps={stepsOf(workspace, workflow.id, steps, scope)}
          level={level}
          added={added}
          onChange={(patch) => change(withRuleAt(workspace, rule.id, scope, patch))}
          onUseAbove={() => change(withoutRuleOverride(workspace, rule.id, scope))}
        />
      );
    }
    if (picked.kind === "lane") {
      const lifecycle = workspace.lifecycles.find((one) => one.id === picked.id);
      if (lifecycle === undefined) return null;
      return (
        <LifecycleFacts
          key={lifecycle.id}
          lifecycle={lifecycle}
          pending={editing.pending?.has(`lifecycle:${lifecycle.id}`) === true}
          named={(lifecycle.scope ?? DEFAULT) === scope}
          onName={(label) => change(withLifecycleLabel(workspace, lifecycle.id, label))}
          onDelete={lifecycleDeletable(lifecycle.id) ? () => remove({ kind: "lane", id: lifecycle.id }) : undefined}
        />
      );
    }
    const { lane, item } = refOf(picked.id);
    if (lane === "workflow") {
      const index = steps.findIndex((one) => one.id === item);
      const step = steps[index];
      if (step === undefined) return null;
      const placed = stepsOf(workspace, workflow.id, steps);
      // A step added at this level is this level's own, so its fields are written to it and not as a change to the level above.
      const added = (place?.added ?? []).find((one) => one.id === step.id);
      const mine = added !== undefined && added.scope === scope;
      /** A step as it reads at a level: its name, words, what ends it and what passes it over. */
      const readAt = (at: string) => {
        const laid = stepsOf(workspace, workflow.id, steps, at).find((one) => one.id === step.id) ?? { id: step.id, label: step.label, col: index };
        const words = layersAt(place, step.id, at).reduce((now, one) => one.words ?? now, step.words);
        return { laid, label: laid.label, words, ends: waitsOf(laid, "ends", step.ends, subjects), passedOver: waitsOf(laid, "passedOver", step.passedOver, subjects), endsOn: laid.endsOn };
      };
      const here = readAt(scope);
      const upper = level === undefined ? undefined : readAt(levelsOf(scope)[1]!);
      const above = (): StepFields => ({ ...(upper === undefined ? {} : { label: upper.label, words: upper.words, ...(upper.ends === undefined ? {} : { ends: upper.ends }), ...(upper.endsOn === undefined ? {} : { endsOn: upper.endsOn }), ...(upper.passedOver === undefined ? {} : { passedOver: upper.passedOver }) }) });
      const layer = (patch: StepFields) => change(withStepLayer(workspace, workflow.id, step.id, scope, patch, above()));
      const waits = (which: "ends" | "passedOver") => (labelledBy: string) => (
        <WaitsPair
          labelledBy={labelledBy}
          subjects={subjects}
          value={here[which]}
          onChange={(next) => {
            const on = endsOnOf(next, subjects);
            if (level === undefined || mine) change(withStepWhen(workspace, workflow.id, placed, step.id, which, next, on));
            else layer(which === "ends" ? { ends: next, ...(on === undefined ? {} : { endsOn: on }) } : { passedOver: next });
          }}
        />
      );
      const own = level !== undefined && !mine && stepDiffers(place, step.id).includes(scope);
      // Where lifecycles and rules are pieces, what the workflow's definition holds is saved by the page: the name and order of its steps on the default. What a level below the default would change of a step, and
      // what ends it or passes it over, have nothing to be kept in, so they are not offered.
      if (pieces) {
        return stepCard({ ...step, label: here.label, words: here.words }, index, {
          ...(level === undefined ? { name: (labelledBy: string) => <Field labelledBy={labelledBy} value={here.label} onCommit={(label) => label !== "" && writeSteps({ kind: "label", id: step.id, label })} /> } : { fixed: true as const }),
          keep: `${workflow.id}/${step.id}/${scope}`,
        });
      }
      return stepCard({ ...step, label: here.label, words: here.words }, index, {
        name: (labelledBy) => <Field labelledBy={labelledBy} value={here.label} onCommit={(label) => label !== "" && (mine ? change(withAddedStep(workspace, workflow.id, step.id, { label })) : level === undefined ? writeSteps({ kind: "label", id: step.id, label }) : layer({ label }))} />,
        ends: waits("ends"),
        ...(step.passedOver === undefined && here.laid.passedOver === undefined ? {} : { passedOver: waits("passedOver") }),
        ...(level === undefined ? {} : { save: async (words: string) => (mine ? change(withAddedStep(workspace, workflow.id, step.id, { words })) : layer({ words }), true), keep: `${workflow.id}/${step.id}/${scope}` }),
        after: own && level !== undefined ? <Changed name={level.name} back={level.back} onUseAbove={() => change(withoutStepLayer(workspace, workflow.id, step.id, scope))} /> : null,
      });
    }
    const lifecycle = workspace.lifecycles.find((one) => one.id === lane);
    const state = lifecycle?.states.find((one) => one.id === item);
    if (lifecycle === undefined || state === undefined) return null;
    return (
      <StateFacts
        lifecycle={lifecycle}
        state={state}
        level={level}
        onLabel={(label) => change(withStateLabelAt(workspace, lane, item, scope, label))}
        onReads={(read) => change(withReads(workspace, lane, item, read))}
        onUseAbove={() => change(withoutStateOverride(workspace, lane, item, scope))}
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


  const workspaces = editing.workspaces?.take === undefined || editing.readOnly === true ? undefined : editing.workspaces;
  const autonomy = editing.readOnly === true ? undefined : editing.autonomy;
  const scopeLabel = scope === DEFAULT ? "Default" : scope === "workspace" ? `All of ${workspace.name}` : (repository?.name ?? scope);
  const formTitle = form === "edit" ? `Edit ${workspace.name}` : "New Workspace";
  /** What the side panel holds beside a pick, and what it is called. */
  const aside = autonomy !== undefined && autonomyOpen ? { title: `Autonomy in ${scopeLabel}`, heading: scopeLabel, crumb: "Autonomy" } : workspaces?.take === "panel" && form !== undefined ? { title: formTitle, heading: formTitle, crumb: "Workspace" } : undefined;
  // A tab picked while the autonomy is open reads it for that tab, so the panel is named for it.
  useEffect(() => {
    if (aside !== undefined) panel.show({ title: aside.title, heading: aside.heading, crumb: aside.crumb });
    // Only a new name asks the panel again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [aside?.title]);
  // A tab that is gone, as the Workspace's when the repository is in none now, is not stood on.
  useEffect(() => {
    if (scope !== DEFAULT && !(scope === "workspace" ? workspace.id !== "" : workspace.repositories.some((one) => one.id === scope))) setScope(DEFAULT);
  }, [scope, workspace.id, workspace.repositories]);

  /** The Workspace form, drawn for the take: its own sheet, or fields in the panel or under a button. `close` puts it away. */
  const workspaceForm = (mode: "new" | "edit", close: () => void) =>
    workspaces === undefined ? null : (
      <WorkspaceForm
        key={`${mode}/${workspace.id}/${workspace.name}`}
        layout={workspaces.take === "sheet" ? "dialog" : "inline"}
        mode={mode}
        repositories={workspaces.repositories}
        current={workspaces.current}
        {...(mode === "edit" ? { name: workspace.name, ticked: workspace.repositories.map((one) => one.id), ...(workspaces.kept === undefined ? {} : { kept: workspaces.kept }) } : {})}
        onSubmit={(name, ids) => {
          if (mode === "new") {
            workspaces.onCreate(name, ids);
            setScope("workspace");
          } else workspaces.onSave(name, ids);
        }}
        onRemove={() => {
          workspaces.onRemove();
          setScope(DEFAULT);
        }}
        onClose={close}
      />
    );
  const putAwayForm = () => {
    setForm(undefined);
    if (workspaces?.take === "panel") panel.close();
  };
  const openForm = (mode: "new" | "edit") => {
    // Under the popover take the layer opens itself, from the button.
    if (workspaces?.take === "popover") return;
    if (workspaces?.take === "panel") {
      setPicked(undefined);
      setMode("fields");
      setAutonomyOpen(false);
    }
    setForm(mode);
  };
  const newWorkspace = (
    <Tooltip label="New Workspace">
      <Button type="button" variant="ghost" size="sm" onClick={() => openForm("new")}>
        <FolderPlus size={16} strokeWidth={2} aria-hidden="true" />
        Workspace
      </Button>
    </Tooltip>
  );
  const editWorkspace = (
    <Tooltip label={`Edit ${workspace.name}`}>
      <Button type="button" variant="ghost" size="sm" iconOnly aria-label={`Edit ${workspace.name}`} onClick={() => openForm("edit")}>
        <Pencil size={14} strokeWidth={2} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
  // A tab that differs from the level above carries the changed glyph, and names what it is changed for.
  const tabs: TabsItem[] = [
    { id: DEFAULT, label: "Default" },
    // A repository that is in no Workspace has no level between the default and itself.
    ...(workspace.id === "" ? [] : [{ id: "workspace", label: `All of ${workspace.name}`, name: workspace.name }]),
    ...workspace.repositories.map((one) => ({ id: one.id, label: one.name, name: one.name })),
  ]
    .map(({ id, label, name }: { id: string; label: string; name?: string }): TabsItem => (name !== undefined && changedAt(workspace, workflow.id, defined, id) ? { id, label, icon: Split, hint: `Changed for ${name}` } : { id, label }))
    // The Workspace's tab carries the way to edit it. Under the popover take the form opens under the pencil.
    .map((item) =>
      item.id === "workspace" && workspaces !== undefined
        ? { ...item, after: workspaces.take === "popover" ? <Popover align="start" label={`Edit ${workspace.name}`} trigger={editWorkspace}>{(close) => workspaceForm("edit", close)}</Popover> : editWorkspace }
        : item,
    );
  const autonomyContent =
    autonomy === undefined || !autonomyOpen ? null : (
      <AutonomyPanel
        take={autonomy.take}
        tab={scope}
        rows={autonomyRows(autonomy.layers, autonomyLevels(scope, { workspace: workspace.id === "" ? undefined : workspace.name, repository: repository?.name }))}
        onSet={(kind, level) => autonomy.onSet(scope, kind, level)}
      />
    );
  const asideContent = autonomyContent ?? (workspaces?.take === "panel" && form !== undefined ? workspaceForm(form, putAwayForm) : null);

  const content = asideContent ?? (picked === undefined ? null : mode === "tell" ? <TellBox keep={`${workflow.id}/${picked.id}`} onSend={tell} /> : inspected);
  /** The rule or lifecycle the pick is of, for the acts on where it is kept. A step is the workflow's. */
  const piece =
    picked === undefined || mode === "tell"
      ? undefined
      : picked.kind === "rule"
        ? ({ kind: "rule", id: picked.id } as const)
        : picked.kind === "lane"
          ? ({ kind: "lifecycle", id: picked.id } as const)
          : refOf(picked.id).lane === "workflow"
            ? undefined
            : ({ kind: "lifecycle", id: refOf(picked.id).lane } as const);
  const acts = piece === undefined ? null : (editing.pieceActs?.(piece, scope) ?? null);

  return (
    <section className="armada-wm-lanes" aria-label="Workflow" style={{ "--lanes": drawn.lanes.length } as CSSProperties}>
      <div className="armada-wm-scope" role="group" aria-label="Runs in">
        <span className="armada-wm-scope__label" aria-hidden="true">
          Runs in
        </span>
        <div className="armada-wm-scope__tabs">
          <Tabs icons items={tabs} value={scope} onChange={setScope} />
        </div>
        <p className="armada-wm-scope__says">{SAYS(scope, workspace.name, repository?.name)}</p>
        {editing.readOnly === true ? null : (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              const made = withLifecycleAdded(workspace, "New lifecycle", scope);
              change(made.workspace);
              pick({ kind: "lane", id: made.id });
            }}
          >
            <Plus size={16} strokeWidth={2} aria-hidden="true" />
            Add lifecycle
          </Button>
        )}
        {workspaces === undefined ? null : workspaces.take === "popover" ? (
          <Popover align="end" label="New Workspace" trigger={newWorkspace}>
            {(close) => workspaceForm("new", close)}
          </Popover>
        ) : (
          newWorkspace
        )}
        {autonomy === undefined ? null : (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            aria-pressed={autonomyOpen}
            onClick={() => {
              setPicked(undefined);
              setMode("fields");
              if (workspaces?.take === "panel") setForm(undefined);
              setAutonomyOpen(true);
            }}
          >
            <SlidersHorizontal size={16} strokeWidth={2} aria-hidden="true" />
            Autonomy
          </Button>
        )}
      </div>
      {above}
      {/* Drawn once the steps are read: lanes that fill in after they open would pan to what arrived. */}
      {steps.length === 0 ? null : <LaneCanvas label={`${workflow.id} workflow`} lanes={drawn.lanes} rules={drawn.rules} picked={picked} onPick={(next) => pick(next)} editing={editing.readOnly === true ? undefined : handles} coveredBy={panel.open ? panel.slot : null} />}
      {content === null || panel.slot === null
        ? null
        : createPortal(
            <div className="armada-wm-inspector">
              {editing.readOnly === true ? <fieldset className="armada-wm-inspector__fixed" disabled>{content}</fieldset> : content}
              {acts}
            </div>,
            panel.slot,
          )}
      {workspaces?.take === "sheet" && form !== undefined ? workspaceForm(form, putAwayForm) : null}
    </section>
  );
}

/** The lifecycles as a tab has them, for the pickers of a rule: the ones that exist there, each with the states that do and the names the tab gives them. */
const lifecyclesOf = (workspace: Workspace, scope: string): RuleFormLifecycle[] =>
  workspace.lifecycles
    .filter((one) => existsAt(one.scope, scope))
    .map((one) => ({ id: one.id, name: one.label, kind: one.kind, states: one.states.filter((state) => stateExistsAt(state, scope)).map((state) => ({ id: state.id, label: stateLabelAt(state, scope) })) }));

/** The mark on a piece whose removal waits on a pull request: a glyph, with the sentence in its tooltip. */
function Pending() {
  return (
    <div className="armada-wm-step__band">
      <span className="armada-wm-step__marks">
        <Tooltip label={REMOVAL_WAITS}>
          <span role="img" aria-label={REMOVAL_WAITS}>
            <Clock size={14} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      </span>
    </div>
  );
}

/** A lifecycle: its name, where Fleet reads its states from, and deleting it. A tab that did not add it reads the name and cannot delete it. */
function LifecycleFacts({ lifecycle, pending, named, onName, onDelete }: { lifecycle: LifecycleDef; pending: boolean; named: boolean; onName: (label: string) => void; onDelete: (() => void) | undefined }) {
  const id = useId();
  return (
    <div className="armada-wm-step">
      {pending ? <Pending /> : null}
      <dl className="armada-wm-step__facts">
        <dt id={`${id}-name`}>Name</dt>
        <dd>{named ? <Field labelledBy={`${id}-name`} value={lifecycle.label} onCommit={(label) => label !== "" && onName(label)} /> : lifecycle.label}</dd>
        <dt>Read from</dt>
        <dd>{lifecycle.from}</dd>
        {onDelete === undefined ? null : (
          <dd className="armada-wm-inspector__changed">
            <Button type="button" variant="ghost" size="sm" onClick={onDelete}>
              <Trash2 size={16} strokeWidth={2} aria-hidden="true" />
              Delete
            </Button>
          </dd>
        )}
      </dl>
    </div>
  );
}

/** What an edit on a tab changes, said under the tabs. */
const SAYS = (scope: string, workspace: string, repository: string | undefined): string =>
  scope === DEFAULT ? "Edits here change every Workspace and repository that keeps the default" : scope === "workspace" ? `Edits here change ${workspace}'s repositories that keep ${workspace}'s` : `Edits here change ${repository ?? scope} only`;

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

/** What differs at a level: the glyph, named for the level, beside what it says. */
function Differs({ name, children }: { name: string; children: ReactNode }) {
  return (
    <span className="armada-wm-inspector__differs">
      <Tooltip label={`Differs in ${name}`}>
        <span role="img" aria-label={`Differs in ${name}`}>
          <Split size={14} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      {children}
    </span>
  );
}

/** What a tab says of a value it changes: where it differs from the level above, named for the tab, and the way back to the one above's. */
function Changed({ name, back, onUseAbove }: { name: string; back: string; onUseAbove: () => void }) {
  return (
    <dd className="armada-wm-inspector__changed">
      <Differs name={name}>{`Changed for ${name}`}</Differs>
      <Button type="button" variant="ghost" size="sm" onClick={onUseAbove}>
        {back}
      </Button>
    </dd>
  );
}

/**
 * A state: its name, where Fleet reads it from, and the words that count as being in it, **for the tab that is picked**. They are that level's own where it changes them, and the level above's
 * until it does; a change says so and can be dropped. Edited on a tab, they change that tab's alone. **Each row has its label once**, and the field in it is named by that label.
 */
function StateFacts({
  lifecycle,
  state,
  level,
  onLabel,
  onReads,
  onUseAbove,
}: {
  lifecycle: LifecycleDef;
  state: StateDef;
  /** The level whose tab is picked; absent on the default. */
  level: Level | undefined;
  onLabel: (label: string) => void;
  onReads: (read: { source: string; scope: string; values: readonly string[] }) => void;
  onUseAbove: () => void;
}) {
  const id = useId();
  const scope = level?.id ?? DEFAULT;
  // A state this tab added is this tab's own, so there is no level above to differ from.
  const mine = state.scope === scope;
  const sources = [...new Set(state.reads.map((one) => one.source))];
  return (
    <div className="armada-wm-step">
      <dl className="armada-wm-step__facts">
        <dt id={`${id}-name`}>Name</dt>
        <dd>
          <Field labelledBy={`${id}-name`} value={stateLabelAt(state, scope)} onCommit={(label) => label !== "" && onLabel(label)} />
        </dd>
        {sources.length === 1 && sources[0] === lifecycle.from ? null : (
          <>
            <dt>Read from</dt>
            <dd>{lifecycle.from}</dd>
          </>
        )}
        {sources.flatMap((source, at) => {
          const above = level === undefined ? [] : (readsAt(state, source, levelsOf(scope)[1]!) ?? []);
          const shown = readsAt(state, source, scope) ?? [];
          return [
            <dt key={`${source}:name`} id={`${id}-${at}`}>{`${source} statuses`}</dt>,
            <dd key={source}>
              <Wrapped
                labelledBy={`${id}-${at}`}
                keep={`workflows:statuses:${lifecycle.id}/${state.id}/${source}/${scope}`}
                value={shown.join(", ")}
                onCommit={(text) => {
                  const values = splitValues(text);
                  // Below the default, the level above's own words are no override.
                  onReads({ source, scope, values: level !== undefined && !mine && values.join(", ") === above.join(", ") ? [] : values });
                }}
              />
            </dd>,
          ];
        })}
        {level === undefined || mine || !stateDiffers(state).includes(scope) ? null : <Changed name={level.name} back={level.back} onUseAbove={onUseAbove} />}
      </dl>
    </div>
  );
}

/**
 * A rule: when it fires, what it checks, what it does and how far it may go alone, **for the tab that is picked**. Every field is that level's own where it changes it, and the level above's
 * until it does; a change says so and can be dropped. **When is two dropdowns**, the step or state and then the moment, so it is changed in two presses. Each row has its label once. A rule a
 * change waiting on Approve would add is shown as facts, and Approve makes it.
 */
function RuleFacts({
  rule,
  workflow,
  workspace,
  steps,
  level,
  added,
  onChange,
  onUseAbove,
}: {
  rule: Rule;
  workflow: string;
  workspace: Workspace;
  steps: readonly { id: string; label: string }[];
  /** The level whose tab is picked; absent on the default. */
  level: Level | undefined;
  /** A rule the proposed change would add is not the Workspace's yet: it is shown, and Approve makes it. */
  added: boolean;
  onChange: (patch: RuleFields) => void;
  onUseAbove: () => void;
}) {
  const id = useId();
  const scope = level?.id ?? DEFAULT;
  const here = ruleAt(rule, scope);
  const writing = !added;
  const kind = here.does[0]?.act ?? "agent";
  const lifecycles = lifecyclesAt(workspace, workflow, scope);
  const groups = fireGroupsOf(steps, lifecycles);
  const from = nodeId(here.from.lane, here.from.item);
  // A node the workflow does not draw still names its place, so the choice shows what the rule has.
  const held: readonly { label?: string; items: readonly Choice[] }[] = groups.some((group) => group.items.some((one) => one.value === from)) ? groups : [{ items: [{ value: from, label: whenSaid({ ...here, at: undefined }, lifecycles, steps) }] }, ...groups];
  const moments = here.from.lane === "workflow" ? MOMENTS.step : MOMENTS.state;
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
              <Field labelledBy={`${id}-name`} value={here.label} onCommit={(label) => label !== "" && onChange({ label })} />
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
              second={{ value: momentOf(here), hidden: "moment", items: moments, onChange: (at) => onChange({ at: at as NonNullable<Rule["at"]> }) }}
            />
          ) : (
            whenSaid(here, workspace.lifecycles, steps)
          )}
        </dd>
        {writing || here.ifs.length > 0 ? (
          <>
            <dt id={`${id}-if`}>If</dt>
            <dd>
              {writing ? (
                <Lines labelledBy={`${id}-if`} keep={`workflows:if:${rule.id}/${scope}`} lines={here.ifs} onCommit={(ifs) => onChange({ ifs })} />
              ) : (
                <ul className="armada-wm-step__ends" aria-labelledby={`${id}-if`}>
                  {here.ifs.map((one) => (
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
            <Lines labelledBy={`${id}-do`} keep={`workflows:do:${rule.id}/${scope}`} lines={here.does.map((one) => one.text)} onCommit={(lines) => onChange({ does: lines.map((text, at) => ({ act: here.does[at]?.act ?? kind, text })) })} />
          ) : (
            <ul className="armada-wm-step__ends" aria-labelledby={`${id}-do`}>
              {here.does.map((one) => (
                <li key={one.text}>{one.text}</li>
              ))}
            </ul>
          )}
        </dd>
        <dt id={`${id}-autonomy`}>Autonomy</dt>
        <dd>
          <RadioGroup labelledBy={`${id}-autonomy`}>
            {(["acts", "asks", "off"] as const).map((one) => (
              <Radio key={one} name={`autonomy:${rule.id}`} checked={here.autonomy === one} disabled={added} onChange={() => onChange({ autonomy: one })}>
                {AUTONOMY_SAID[one]}
              </Radio>
            ))}
          </RadioGroup>
        </dd>
        {level === undefined || !(rule.overrides ?? []).some((one) => one.scope === scope) ? null : <Changed name={level.name} back={level.back} onUseAbove={onUseAbove} />}
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
