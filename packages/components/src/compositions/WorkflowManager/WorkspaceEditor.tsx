import { ListPlus } from "lucide-react";
import { useMemo, useState } from "react";
import type { ReactNode } from "react";

import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tabs } from "../../primitives/Tabs/Tabs";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { appliesIn, editorLanes } from "../LaneCanvas/build";
import { LaneCanvas } from "../LaneCanvas/LaneCanvas";
import type { LanePick } from "../LaneCanvas/LaneCanvas";
import { whenSaid } from "../LaneCanvas/model";
import type { Autonomy, Kept, Lifecycle, LifecycleState, Rule, Workspace } from "../LaneCanvas/model";
import type { ManagedStep, ManagedWorkflow } from "./types";

/**
 * The Workspace a workflow is edited in: its steps on top of a lane for each lifecycle, rules as edges, and an inspector under the canvas
 * for whatever is picked. **A mock's editor**: no Fleet serves a Workspace yet. A scope picks the Workspace or one repository, and the
 * lanes show the rules and the tracker values that apply there. The words a step is told stay the step card, drawn as the inspector.
 *
 * Keys are the canvas's own (`LaneCanvas`): arrows step between nodes, `R` and `Shift R` go round the rules, `Esc` lets go of the pick.
 */
export type WorkspaceEditing = {
  value: Workspace;
  /** A rule's autonomy changed. Fleet would refuse it on a copy that is not the person's. */
  onAutonomy: (rule: string, autonomy: Autonomy) => void;
};

const NONE: readonly never[] = [];

const KEPT_SAID: Readonly<Record<Kept, string>> = { armada: "Armada's", yours: "Yours", repository: "Repository's" };
const AUTONOMY_SAID: Readonly<Record<Autonomy, string>> = { acts: "Acts", asks: "Asks", off: "Off" };

export function WorkspaceEditor({
  editing,
  workflow,
  editable,
  proposed = NONE,
  stepCard,
}: {
  editing: WorkspaceEditing;
  workflow: ManagedWorkflow;
  /** Only the person's own copy changes a rule. */
  editable: boolean;
  /** The rules a change waiting on Approve would add, drawn as a diff. */
  proposed?: readonly Rule[];
  /** The step card for the step picked: its words, what ends it, what drifted. */
  stepCard: (step: ManagedStep, at: number) => ReactNode;
}) {
  const workspace = editing.value;
  const [scope, setScope] = useState("workspace");
  const [picked, setPicked] = useState<LanePick | undefined>();
  const drawn = useMemo(() => editorLanes({ workspace, workflow: workflow.id, steps: workflow.steps, scope, proposed }), [workspace, workflow.id, workflow.steps, scope, proposed]);
  const scopes = useMemo(() => [{ id: "workspace", label: workspace.name }, ...workspace.repositories.map((one) => ({ id: one.id, label: one.name }))], [workspace]);
  const repositoryName = (id: string) => workspace.repositories.find((one) => one.id === id)?.name ?? id;

  const inspected = (() => {
    if (picked === undefined) return null;
    if (picked.kind === "rule") {
      const rule = [...workspace.rules, ...proposed].find((one) => one.id === picked.id);
      return rule === undefined || !appliesIn(rule, scope) ? null : (
        <RuleFacts rule={rule} workspace={workspace} steps={workflow.steps} editable={editable && !proposed.includes(rule)} repositoryName={repositoryName} onAutonomy={editing.onAutonomy} added={proposed.includes(rule)} />
      );
    }
    const at = picked.id.indexOf("/");
    const lane = picked.id.slice(0, at);
    const item = picked.id.slice(at + 1);
    if (lane === "workflow") {
      const index = workflow.steps.findIndex((one) => one.id === item);
      const step = workflow.steps[index];
      return step === undefined ? null : stepCard(step, index);
    }
    const lifecycle = workspace.lifecycles.find((one) => one.id === lane);
    const state = lifecycle?.states.find((one) => one.id === item);
    return lifecycle === undefined || state === undefined ? null : <StateFacts lifecycle={lifecycle} state={state} scope={scope} repositoryName={repositoryName} />;
  })();

  return (
    <section className="armada-wm-lanes" aria-label="Lanes">
      <div role="group" aria-label="Scope" className="armada-wm-lanes__scope">
        <Tabs
          items={scopes}
          value={scope}
          onChange={(next) => {
            setScope(next);
            setPicked(undefined);
          }}
        />
      </div>
      <LaneCanvas label={`Lanes of ${workflow.id}`} lanes={drawn.lanes} rules={drawn.rules} picked={picked} onPick={setPicked} />
      {inspected === null ? null : (
        <aside className="armada-wm-inspector" aria-label="Inspector">
          {inspected}
        </aside>
      )}
    </section>
  );
}

/** A state: where Fleet reads it from and the values that count as being in it, for the Workspace and for the repository that differs. */
function StateFacts({ lifecycle, state, scope, repositoryName }: { lifecycle: Lifecycle; state: LifecycleState; scope: string; repositoryName: (id: string) => string }) {
  const overridden = new Set(state.reads.filter((one) => one.scope === scope && scope !== "workspace").map((one) => one.source));
  const reads = state.reads.filter((one) => (scope === "workspace" ? true : one.scope === scope || (one.scope === "workspace" && !overridden.has(one.source))));
  return (
    <div className="armada-wm-step">
      <div className="armada-wm-step__band">
        <span className="armada-wm-step__label">{`${lifecycle.label}: ${state.label}`}</span>
      </div>
      <dl className="armada-wm-step__facts">
        <dt>Read from</dt>
        <dd>{lifecycle.from}</dd>
        {reads.map((one) => (
          <FactRow key={`${one.source}:${one.scope}`} name={one.scope === "workspace" ? one.source : `${one.source}, ${repositoryName(one.scope)}`}>
            <code className="armada-wm-inspector__values">{one.values.join(", ")}</code>
          </FactRow>
        ))}
      </dl>
    </div>
  );
}

function FactRow({ name, children }: { name: string; children: ReactNode }) {
  return (
    <>
      <dt>{name}</dt>
      <dd>{children}</dd>
    </>
  );
}

/** A rule: when it fires, what it checks, what it does and how far it may go alone. */
function RuleFacts({
  rule,
  workspace,
  steps,
  editable,
  added,
  repositoryName,
  onAutonomy,
}: {
  rule: Rule;
  workspace: Workspace;
  steps: readonly { id: string; label: string }[];
  editable: boolean;
  added: boolean;
  repositoryName: (id: string) => string;
  onAutonomy: (rule: string, autonomy: Autonomy) => void;
}) {
  return (
    <div className="armada-wm-step" data-added={added || undefined}>
      <div className="armada-wm-step__band">
        <span className="armada-wm-step__label">{rule.label}</span>
        {added ? (
          <span className="armada-wm-step__marks">
            <Tooltip label="Added by the proposed change">
              <span role="img" aria-label="Added by the proposed change">
                <ListPlus size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          </span>
        ) : null}
      </div>
      <dl className="armada-wm-step__facts">
        <dt>When</dt>
        <dd>{whenSaid(rule, workspace.lifecycles, steps)}</dd>
        {rule.ifs.length === 0 ? null : (
          <>
            <dt>If</dt>
            <dd>
              <ul className="armada-wm-step__ends" aria-label="If">
                {rule.ifs.map((one) => (
                  <li key={one}>{one}</li>
                ))}
              </ul>
            </dd>
          </>
        )}
        <dt>Do</dt>
        <dd>
          <ul className="armada-wm-step__ends" aria-label="Do">
            {rule.does.map((one) => (
              <li key={one.text}>{one.text}</li>
            ))}
          </ul>
        </dd>
        <dt>Autonomy</dt>
        <dd>
          <RadioGroup label="Autonomy">
            {(["acts", "asks", "off"] as const).map((one) => (
              <Radio key={one} name={`autonomy:${rule.id}`} checked={rule.autonomy === one} disabled={!editable} onChange={() => onAutonomy(rule.id, one)}>
                {AUTONOMY_SAID[one]}
              </Radio>
            ))}
          </RadioGroup>
        </dd>
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
