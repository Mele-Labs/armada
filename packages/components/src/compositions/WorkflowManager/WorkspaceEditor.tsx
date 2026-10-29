import { ListPlus, Split } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { ReactNode } from "react";
import { createPortal } from "react-dom";

import { Input } from "../../primitives/Input/Input";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { editorLanes, stepsOf } from "../LaneCanvas/build";
import { ruleBetween, uniqueId, withNewRule, withOverride, withReads, withRule, withStateAfter, withStateLabel, withStepAfter, withStepsOrder, withoutRule, withoutState, withoutStep } from "../LaneCanvas/edit";
import { LaneCanvas } from "../LaneCanvas/LaneCanvas";
import type { LaneEditing, LanePick } from "../LaneCanvas/LaneCanvas";
import { nodeId, whenSaid } from "../LaneCanvas/model";
import type { Autonomy, Kept, LifecycleDef, Rule, StateDef, Workspace } from "../LaneCanvas/model";
import { SessionComposer } from "../SessionComposer/SessionComposer";
import type { SentFromComposer } from "../SessionComposer/SessionComposer";
import type { ManagedStep, ManagedWorkflow, WorkflowActAnswer, WorkflowRef } from "./types";

/**
 * The Workspace a workflow is edited in: its steps on top of a lane for each lifecycle it uses, and rules as edges. **A mock's editor**: no Fleet serves a Workspace yet.
 * The lanes show the Workspace, and where a repository differs a node or a rule says so (a mark) and its panel lists each repository's value beside the Workspace's.
 *
 * **What is picked opens in the app's side panel**, drawn into the slot the host gives (`LanePanel`), since the panel is the host's and this is a component: the facts, then the same
 * fields to edit, or a box to tell an agent. **Hand editing is on the canvas**: a toolbar over what is picked, a connector from one node to another, and on Armada's copy the first
 * edit makes the person's own, which the toolbar says.
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

const KEPT_SAID: Readonly<Record<Kept, string>> = { armada: "Armada's", yours: "Yours", repository: "Repository's" };
const AUTONOMY_SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

const refOf = (id: string): { lane: string; item: string } => ({ lane: id.slice(0, id.indexOf("/")), item: id.slice(id.indexOf("/") + 1) });
const splitValues = (text: string): string[] => text.split(",").map((one) => one.trim()).filter((one) => one !== "");

type Mode = "read" | "edit" | "tell";

export function WorkspaceEditor({
  editing,
  workflow,
  editable,
  proposed = NONE,
  stepCard,
  panel,
  onMakeCopy,
  onTell,
  onSteps,
}: {
  editing: WorkspaceEditing;
  workflow: ManagedWorkflow;
  /** The person's own copy: Armada's is read only, and the first edit makes the person's own. */
  editable: boolean;
  /** The rules a change waiting on Approve would add, drawn as a diff. */
  proposed?: readonly Rule[];
  /** The step card for the step picked: its words, what ends it, what drifted. */
  stepCard: (step: ManagedStep, at: number) => ReactNode;
  panel: LanePanel;
  onMakeCopy: () => void;
  /** An agent told a change about what is picked. */
  onTell: (request: string) => Promise<WorkflowActAnswer>;
  onSteps: (change: StepsChange) => void;
}) {
  const workspace = editing.value;
  const [picked, setPicked] = useState<LanePick | undefined>();
  const [mode, setMode] = useState<Mode>("read");
  const drawn = useMemo(() => editorLanes({ workspace, workflow: workflow.id, steps: workflow.steps, proposed }), [workspace, workflow.id, workflow.steps, proposed]);
  const repositoryName = (id: string) => workspace.repositories.find((one) => one.id === id)?.name ?? id;
  const change = (next: Workspace) => editing.onChange(next);
  const steps = workflow.steps;

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

  const pick = (next: LanePick | undefined, then: Mode = "read") => {
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
      setMode("read");
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

  const addAfter = (at: LanePick) => {
    if (at.kind !== "node") return;
    const { lane, item } = refOf(at.id);
    if (lane === "workflow") {
      const id = uniqueId("New step", steps.map((one) => one.id));
      change(withStepAfter(workspace, workflow.id, stepsOf(workspace, workflow.id, steps), item, id));
      onSteps({ kind: "add", after: item, id, label: "New step" });
      pick({ kind: "node", id: nodeId("workflow", id) }, "edit");
      return;
    }
    const made = withStateAfter(workspace, workflow.id, lane, item, "New state");
    change(made.workspace);
    pick({ kind: "node", id: nodeId(lane, made.id) }, "edit");
  };

  const remove = (at: LanePick) => {
    if (at.kind === "rule") change(withoutRule(workspace, at.id));
    else {
      const { lane, item } = refOf(at.id);
      if (lane === "workflow") {
        change(withoutStep(workspace, workflow.id, item));
        onSteps({ kind: "remove", id: item });
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
      onSteps({ kind: "order", ids });
      pick({ kind: "node", id: to });
      return;
    }
    const rule = ruleBetween(workspace, workflow.id, a, b, { from: names.get(from)?.label ?? a.item, to: names.get(to)?.label ?? b.item, lane: names.get(to)?.lane ?? b.lane });
    change(withNewRule(workspace, rule));
    pick({ kind: "rule", id: rule.id }, "edit");
  };

  const handles: LaneEditing = {
    armada: !editable,
    editing: mode === "edit",
    onEdit: () => setMode((now) => (now === "edit" ? "read" : "edit")),
    onTell: () => setMode("tell"),
    onAddAfter: addAfter,
    onDelete: remove,
    onMakeCopy,
    onConnect: connect,
  };

  const inspected = (() => {
    if (picked === undefined) return null;
    const open = editable && mode === "edit";
    if (picked.kind === "rule") {
      const rule = [...workspace.rules, ...proposed].find((one) => one.id === picked.id);
      if (rule === undefined) return null;
      const added = proposed.includes(rule);
      return (
        <RuleFacts
          rule={rule}
          workspace={workspace}
          steps={steps}
          editing={open && !added}
          repositoryName={repositoryName}
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
          {open ? <Field label="Step name" value={step.label} onCommit={(label) => onSteps({ kind: "label", id: step.id, label })} /> : null}
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
        editing={open}
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

/** Lines of text that edit a stored list, said where the box is left. */
function Lines({ label, lines, onCommit }: { label: string; lines: readonly string[]; onCommit: (next: string[]) => void }) {
  const [text, setText] = useState(lines.join("\n"));
  useEffect(() => setText(lines.join("\n")), [lines]);
  return (
    <Textarea
      label={label}
      rows={Math.max(2, lines.length)}
      keep={false}
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

/**
 * A state: where Fleet reads it from, and the values that count as being in it for the Workspace and for each repository beside it. A repository that reads another value is marked
 * and named; one that reads the Workspace's is drawn quiet.
 */
function StateFacts({
  lifecycle,
  state,
  workspace,
  editing,
  onLabel,
  onReads,
}: {
  lifecycle: LifecycleDef;
  state: StateDef;
  workspace: Workspace;
  editing: boolean;
  onLabel: (label: string) => void;
  onReads: (read: { source: string; scope: string; values: readonly string[] }) => void;
}) {
  const sources = [...new Set(state.reads.map((one) => one.source))];
  const valuesOf = (source: string, scope: string) => state.reads.find((one) => one.source === source && one.scope === scope)?.values;
  return (
    <div className="armada-wm-step">
      {editing ? (
        <div className="armada-wm-inspector__edit">
          <Field label="State name" value={state.label} onCommit={(label) => label !== "" && onLabel(label)} />
        </div>
      ) : null}
      <dl className="armada-wm-step__facts">
        <dt>Read from</dt>
        <dd>{lifecycle.from}</dd>
        {sources.flatMap((source) => {
          const own = valuesOf(source, "workspace") ?? [];
          return [
            <dt key={`${source}:workspace:name`}>{sources.length > 1 ? `${source}` : "Workspace"}</dt>,
            <dd key={`${source}:workspace`}>
              {editing ? (
                <Field label={`${source} values for the Workspace`} mono value={own.join(", ")} onCommit={(text) => onReads({ source, scope: "workspace", values: splitValues(text) })} />
              ) : (
                <code className="armada-wm-inspector__values">{own.join(", ")}</code>
              )}
            </dd>,
            ...workspace.repositories.flatMap((repository) => {
              const differs = valuesOf(source, repository.id);
              return [
                <dt key={`${source}:${repository.id}:name`}>{repository.name}</dt>,
                <dd key={`${source}:${repository.id}`}>
                  {editing ? (
                    <Field
                      label={`${source} values for ${repository.name}`}
                      mono
                      placeholder={own.join(", ")}
                      value={(differs ?? []).join(", ")}
                      onCommit={(text) => onReads({ source, scope: repository.id, values: splitValues(text) })}
                    />
                  ) : differs === undefined ? (
                    <code className="armada-wm-inspector__values" data-same="">
                      {own.join(", ")}
                    </code>
                  ) : (
                    <Differs repository={repository.name}>
                      <code className="armada-wm-inspector__values">{differs.join(", ")}</code>
                    </Differs>
                  )}
                </dd>,
              ];
            }),
          ];
        })}
      </dl>
    </div>
  );
}

/** A rule: when it fires, what it checks, what it does and how far it may go alone, in the Workspace and in each repository beside it. */
function RuleFacts({
  rule,
  workspace,
  steps,
  editing,
  added,
  repositoryName,
  onChange,
  onOverride,
}: {
  rule: Rule;
  workspace: Workspace;
  steps: readonly { id: string; label: string }[];
  editing: boolean;
  added: boolean;
  repositoryName: (id: string) => string;
  onChange: (patch: { label?: string; at?: "starts" | "ends"; ifs?: readonly string[]; does?: Rule["does"]; autonomy?: Autonomy }) => void;
  onOverride: (scope: string, autonomy: Autonomy) => void;
}) {
  const overrideOf = (scope: string): Autonomy | undefined => rule.overrides?.find((one) => one.scope === scope)?.autonomy;
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
      {editing ? (
        <div className="armada-wm-inspector__edit">
          <Field label="Rule name" value={rule.label} onCommit={(label) => label !== "" && onChange({ label })} />
        </div>
      ) : null}
      <dl className="armada-wm-step__facts">
        <dt>When</dt>
        <dd>
          {editing && rule.from.lane === "workflow" ? (
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
        {editing || rule.ifs.length > 0 ? (
          <>
            <dt>If</dt>
            <dd>
              {editing ? (
                <Lines label="If" lines={rule.ifs} onCommit={(ifs) => onChange({ ifs })} />
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
          {editing ? (
            <Lines label="Do" lines={rule.does.map((one) => one.text)} onCommit={(lines) => onChange({ does: lines.map((text, at) => ({ act: rule.does[at]?.act ?? kind, text })) })} />
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
              <Radio key={one} name={`autonomy:${rule.id}`} checked={rule.autonomy === one} disabled={!editing} onChange={() => onChange({ autonomy: one })}>
                {AUTONOMY_SAID[one]}
              </Radio>
            ))}
          </RadioGroup>
        </dd>
        {workspace.repositories.length === 0 ? null : (
          <>
            <dt>By repository</dt>
            <dd>
              <ul className="armada-wm-inspector__repositories" aria-label="Autonomy by repository">
                {workspace.repositories.map((repository) => {
                  const own = overrideOf(repository.id);
                  return (
                    <li key={repository.id}>
                      <span className="armada-wm-inspector__repository">{repository.name}</span>
                      {editing ? (
                        <Select aria-label={`Autonomy in ${repository.name}`} value={own ?? "same"} onChange={(event) => onOverride(repository.id, event.target.value === "same" ? rule.autonomy : (event.target.value as Autonomy))}>
                          <option value="same">{`Same: ${AUTONOMY_SAID[rule.autonomy]}`}</option>
                          {(["acts", "asks", "off"] as const)
                            .filter((one) => one !== rule.autonomy)
                            .map((one) => (
                              <option key={one} value={one}>
                                {AUTONOMY_SAID[one]}
                              </option>
                            ))}
                        </Select>
                      ) : own === undefined ? (
                        <span className="armada-wm-inspector__same">{AUTONOMY_SAID[rule.autonomy]}</span>
                      ) : (
                        <Differs repository={repository.name}>{AUTONOMY_SAID[own]}</Differs>
                      )}
                    </li>
                  );
                })}
              </ul>
            </dd>
          </>
        )}
        <dt>Copy</dt>
        <dd>{KEPT_SAID[rule.kept]}</dd>
        <dt>Applies in</dt>
        <dd>{rule.scope === "workspace" ? workspace.name : repositoryName(rule.scope)}</dd>
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
