import { ListPlus, Split } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { editorLanes, stepsOf } from "../LaneCanvas/build";
import { ruleBetween, uniqueId, withNewRule, withOverride, withReads, withRule, withStateAfter, withStateLabel, withStepAfter, withStepsOrder, withoutRule, withoutState, withoutStep } from "../LaneCanvas/edit";
import { LaneCanvas } from "../LaneCanvas/LaneCanvas";
import type { LaneEditing, LanePick } from "../LaneCanvas/LaneCanvas";
import { nodeId, whenSaid } from "../LaneCanvas/model";
import type { Autonomy, LifecycleDef, Rule, StateDef, Workspace } from "../LaneCanvas/model";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { SentFromComposer } from "../SessionComposer/SessionComposer";
import type { ManagedStep, ManagedWorkflow, WorkflowActAnswer, WorkflowRef } from "./types";

/**
 * The Workspace a workflow is edited in: its steps on top of a lane for each lifecycle it uses, and rules as edges. **A mock's editor**: no Fleet serves a Workspace yet.
 * **Runs in** is a row of tabs over the canvas: All of the Workspace is the base, and each repository is the base with its own overrides, as a site builder edits a breakpoint. A repository's tab draws the
 * lanes as the workflow runs there, marks the nodes that differ there, and what is edited on it is an override for that repository alone. The panel shows the picked tab's one value, and says where a
 * repository's differs from the base. **Under the tabs is the box that tells an agent what to change** (`above`), always on show.
 *
 * **What is picked opens in the app's side panel**, drawn into the slot the host gives (`LanePanel`), since the panel is the host's and this is a component: its settings as live fields, saved as they
 * are changed, or a box to tell an agent. **There is no read only state and no mode to enter.** Hand editing is on the canvas too: a toolbar over what is picked, whose Edit goes to the fields, and a connector from
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

/** The width the editor keeps beside a panel before the list gives up room: the title's acts, and the tabs on a line each. */
const NARROW = 420;
const NOBODY = () => undefined;

const AUTONOMY_SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

const refOf = (id: string): { lane: string; item: string } => ({ lane: id.slice(0, id.indexOf("/")), item: id.slice(id.indexOf("/") + 1) });
const splitValues = (text: string): string[] => text.split(",").map((one) => one.trim()).filter((one) => one !== "");

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
  /** The step card for the step picked: its words, what ends it, what drifted. */
  stepCard: (step: ManagedStep, at: number) => ReactNode;
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
  const section = useRef<HTMLElement>(null);
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

  // **The panel takes room, and the editor gives it.** The app's side panel floats over the right of the window, so the column the editor stands in is narrowed by what it covers (and the list beside
  // it gives up room when what is left is narrow), and nothing the panel lies over is left to be pressed beneath it.
  useEffect(() => {
    const root = section.current?.closest<HTMLElement>(".armada-wm");
    if (root === null || root === undefined) return;
    const sheet = panel.open ? panel.slot?.closest<HTMLElement>("[data-side]") : null;
    const release = () => {
      root.style.removeProperty("--armada-wm-covered");
      delete root.dataset.covered;
      delete root.dataset.crowded;
    };
    if (sheet === null || sheet === undefined) return release();
    let live = true;
    let settled = false;
    const measure = () => {
      if (!live || !settled) return;
      const bounds = root.getBoundingClientRect();
      const covered = Math.max(0, Math.round(bounds.right - sheet.getBoundingClientRect().left));
      if (covered === 0) return release();
      const style = getComputedStyle(root);
      const list = parseFloat(style.getPropertyValue("--w-guide-list")) || 0;
      const gap = parseFloat(style.columnGap) || 0;
      root.style.setProperty("--armada-wm-covered", `${covered}px`);
      root.dataset.covered = "";
      if (bounds.width - covered - list - gap < NARROW) root.dataset.crowded = "";
      else delete root.dataset.crowded;
    };
    // The panel travels in from the edge: it is read once it has settled, and again as it is resized or the window is.
    void Promise.allSettled(sheet.getAnimations().map((one) => one.finished)).then(() => {
      settled = true;
      measure();
    });
    const watch = new ResizeObserver(measure);
    watch.observe(sheet);
    watch.observe(root);
    return () => {
      live = false;
      watch.disconnect();
      release();
    };
  }, [panel.open, panel.slot]);

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
      return (
        <>
          <div className="armada-wm-inspector__edit">
            <Field label="Step name" value={step.label} onCommit={(label) => label !== "" && writeSteps({ kind: "label", id: step.id, label })} />
          </div>
          {stepCard(step, index)}
        </>
      );
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
    <section ref={section} className="armada-wm-lanes" aria-label="Lanes">
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

/** A field that edits a stored value and says it where it is left, or on Enter. */
function Field({ label, value, onCommit, mono = false, placeholder }: { label: string; value: string; onCommit: (next: string) => void; mono?: boolean; placeholder?: string }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const next = text.trim();
    if (next !== value) onCommit(next);
  };
  return (
    <Input
      label={label}
      mono={mono}
      value={text}
      {...(placeholder === undefined ? {} : { placeholder })}
      onChange={(event) => setText(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
      }}
    />
  );
}

/** Lines of text that edit a stored list, said where the box is left. `keep` names what the lines are about, so a line half typed is there when the panel comes back. */
function Lines({ label, keep, lines, onCommit }: { label: string; keep: string; lines: readonly string[]; onCommit: (next: string[]) => void }) {
  const [text, setText] = useState(lines.join("\n"));
  useEffect(() => setText(lines.join("\n")), [lines]);
  return (
    <Textarea
      label={label}
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
 * repository's, which are the base's until it overrides them, and an override says so and can be dropped. Edited on a tab, they change that tab's alone.
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
  const sources = [...new Set(state.reads.map((one) => one.source))];
  const valuesOf = (source: string, at: string) => state.reads.find((one) => one.source === source && one.scope === at)?.values;
  return (
    <div className="armada-wm-step">
      {scope === undefined ? (
        <div className="armada-wm-inspector__edit">
          <Field label="State name" value={state.label} onCommit={(label) => label !== "" && onLabel(label)} />
        </div>
      ) : null}
      <dl className="armada-wm-step__facts">
        {sources.length === 1 && sources[0] === lifecycle.from ? null : (
          <>
            <dt>Read from</dt>
            <dd>{lifecycle.from}</dd>
          </>
        )}
        {sources.flatMap((source) => {
          const base = valuesOf(source, "workspace") ?? [];
          const own = scope === undefined ? undefined : valuesOf(source, scope.id);
          const shown = own ?? base;
          return [
            <dt key={`${source}:name`}>{`${source} statuses`}</dt>,
            <dd key={source}>
              <Field
                label={scope === undefined ? `${source} statuses` : `${source} statuses for ${scope.name}`}
                mono
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
 * until it is overridden, and an override says so and can be dropped. Its words and its place are the base's, edited there.
 */
function RuleFacts({
  rule,
  workspace,
  steps,
  scope,
  added,
  onChange,
  onOverride,
}: {
  rule: Rule;
  workspace: Workspace;
  steps: readonly { id: string; label: string }[];
  /** The repository whose tab is picked; absent on the base. */
  scope: { id: string; name: string } | undefined;
  /** A rule the proposed change would add is not the Workspace's yet: it is shown, and Approve makes it. */
  added: boolean;
  onChange: (patch: { label?: string; at?: "starts" | "ends"; ifs?: readonly string[]; does?: Rule["does"]; autonomy?: Autonomy }) => void;
  onOverride: (scope: string, autonomy: Autonomy) => void;
}) {
  const own = scope === undefined ? undefined : rule.overrides?.find((one) => one.scope === scope.id)?.autonomy;
  const autonomy = own ?? rule.autonomy;
  // Its name, its place and its lines are the base's: they are edited there, not per repository.
  const writing = scope === undefined && !added;
  const kind = rule.does[0]?.act ?? "agent";
  return (
    <div className="armada-wm-step" data-added={added || undefined}>
      {added ? (
        <div className="armada-wm-step__band">
          <span className="armada-wm-step__label">{rule.label}</span>
          <span className="armada-wm-step__marks">
            <Tooltip label="Added by the proposed change">
              <span role="img" aria-label="Added by the proposed change">
                <ListPlus size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          </span>
        </div>
      ) : null}
      {writing ? (
        <div className="armada-wm-inspector__edit">
          <Field label="Rule name" value={rule.label} onCommit={(label) => label !== "" && onChange({ label })} />
        </div>
      ) : null}
      <dl className="armada-wm-step__facts">
        <dt>When</dt>
        <dd>
          {writing && rule.from.lane === "workflow" ? (
            <RadioGroup label="When">
              {(["starts", "ends"] as const).map((one) => (
                <Radio key={one} name={`at:${rule.id}`} checked={(rule.at ?? "starts") === one} onChange={() => onChange({ at: one })}>
                  {`${steps.find((each) => each.id === rule.from.item)?.label ?? rule.from.item} ${one}`}
                </Radio>
              ))}
            </RadioGroup>
          ) : (
            whenSaid(rule, workspace.lifecycles, steps)
          )}
        </dd>
        {writing || rule.ifs.length > 0 ? (
          <>
            <dt>If</dt>
            <dd>
              {writing ? (
                <Lines label="If" keep={`workflows:if:${rule.id}`} lines={rule.ifs} onCommit={(ifs) => onChange({ ifs })} />
              ) : (
                <ul className="armada-wm-step__ends" aria-label="If">
                  {rule.ifs.map((one) => (
                    <li key={one}>{one}</li>
                  ))}
                </ul>
              )}
            </dd>
          </>
        ) : null}
        <dt>Do</dt>
        <dd>
          {writing ? (
            <Lines label="Do" keep={`workflows:do:${rule.id}`} lines={rule.does.map((one) => one.text)} onCommit={(lines) => onChange({ does: lines.map((text, at) => ({ act: rule.does[at]?.act ?? kind, text })) })} />
          ) : (
            <ul className="armada-wm-step__ends" aria-label="Do">
              {rule.does.map((one) => (
                <li key={one.text}>{one.text}</li>
              ))}
            </ul>
          )}
        </dd>
        <dt>Autonomy</dt>
        <dd>
          <RadioGroup label="Autonomy">
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
