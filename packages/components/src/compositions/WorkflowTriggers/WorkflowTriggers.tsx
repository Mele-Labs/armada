import { Briefcase, FolderGit2, Check, CircleDot, Package, SquarePlus, Trash2, Webhook, X } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Source } from "../WorkflowCreator/def";
import type { WorkflowStepCardProps } from "../WorkflowStepCard/WorkflowStepCard";
import {
  EVERY,
  MOCK_COMMANDS,
  MOCK_SKILLS,
  PLACE_DIR,
  whenSaid,
  type FiredTrigger,
  type Trigger,
  type TriggerKind,
  type TriggerWhen,
  type Inserted,
  type InsertedKind,
  type ResolvedTrigger,
} from "./triggers";

/**
 * Triggers: a script or a skill that fires at a moment in a Job. The rows are the
 * list the Workflow creator draws twice, beside the workflows and on a step,
 * and each opens the same fields; `FiredTriggers` is what Job detail draws once
 * they have fired. A step added to one Job is the same thing for that Job
 * alone. Mock only: no Fleet behind it.
 */

/** What each place is, said once on its mark. */
const PLACE: Record<Source, { Glyph: LucideIcon; said: string }> = {
  carried: { Glyph: Package, said: "Armada default" },
  repository: { Glyph: FolderGit2, said: "This repository" },
  kit: { Glyph: Briefcase, said: "This machine" },
};

function PlaceMark({ place }: { place: Source }) {
  const { Glyph, said } = PLACE[place];
  return (
    <Tooltip label={said}>
      <span className="armada-triggers__mark" role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The triggers as rows, each marked by where it is set. A shadowed trigger is drawn quiet. */
export function TriggerRows({ triggers, label, onOpen }: { triggers: readonly ResolvedTrigger[]; label: string; onOpen: (id: string) => void }) {
  return (
    <ul className="armada-triggers" aria-label={label}>
      {triggers.map((one) => {
        const over = one.overriddenBy === undefined ? undefined : `Overridden by ${PLACE[one.overriddenBy].said.toLowerCase()}`;
        return (
          <li key={one.id} className="armada-triggers__row" data-state={over === undefined ? undefined : "overridden"}>
            <button
              type="button"
              aria-label={`${one.name}, ${whenSaid(one)}, ${PLACE[one.place].said.toLowerCase()}${over === undefined ? "" : ", overridden"}`}
              onClick={() => onOpen(one.id)}
            >
              <PlaceMark place={one.place} />
              <span className="armada-triggers__name">{one.name}</span>
              <span className="armada-triggers__when">{whenSaid(one)}</span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

/** The two switches a trigger and an added step share. */
function Fails({ block, repair, onChange }: { block: boolean; repair: boolean; onChange: (next: { block?: boolean; repair?: boolean }) => void }) {
  return (
    <div className="armada-triggers__fails" role="group" aria-label="If it fails">
      <span className="armada-triggers__eyebrow">If it fails</span>
      <Switch checked={block} onChange={(event) => onChange({ block: event.target.checked })}>
        Block the Job
      </Switch>
      <Switch checked={repair} onChange={(event) => onChange({ repair: event.target.checked })}>
        Self repair
      </Switch>
    </div>
  );
}

/** The fields of one trigger, in the app's overlay panel. */
export function TriggerFields({
  trigger,
  workflow,
  steps,
  onChange,
  onRemove,
}: {
  trigger: Trigger;
  /** The workflow open beside the panel, which "This workflow" means. Absent where none is. */
  workflow: string | undefined;
  /** That workflow's steps. */
  steps: readonly string[];
  onChange: (next: Partial<Trigger>) => void;
  /** Absent where the trigger is not saved yet. */
  onRemove?: () => void;
}) {
  const names = trigger.kind === "command" ? MOCK_COMMANDS : MOCK_SKILLS;
  const held = names.includes(trigger.name as never) ? names : [...names, trigger.name];
  const applies = trigger.applies === EVERY || trigger.applies === workflow ? [] : [trigger.applies];
  const stepped = trigger.when !== "pr_opened";
  return (
    <>
      <Select label="When" value={trigger.when} onChange={(event) => onChange({ when: event.target.value as TriggerWhen })}>
        <option value="starts">A step starts</option>
        <option value="passes">A step passes</option>
        <option value="pr_opened">PR opened</option>
      </Select>
      {stepped ? (
        trigger.applies !== EVERY && workflow === trigger.applies ? (
          <Select label="Step" value={trigger.step} onChange={(event) => onChange({ step: event.target.value })}>
            <option value="" />
            {steps.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
        ) : (
          <Input label="Step" mono value={trigger.step} onChange={(event) => onChange({ step: event.target.value })} />
        )
      ) : null}
      <Select label="Applies to" value={trigger.applies} onChange={(event) => onChange({ applies: event.target.value })}>
        <option value={EVERY}>Every workflow</option>
        {workflow === undefined ? null : <option value={workflow}>{`This workflow, ${workflow}`}</option>}
        {applies.map((one) => (
          <option key={one} value={one}>{`Workflow ${one}`}</option>
        ))}
      </Select>
      <Select label="Set in" value={trigger.place} onChange={(event) => onChange({ place: event.target.value as Source })}>
        {trigger.place === "carried" ? <option value="carried">Armada default</option> : null}
        <option value="repository">Repository</option>
        <option value="kit">This machine</option>
      </Select>
      <span className="armada-triggers__fact">{`${PLACE_DIR[trigger.place]}/triggers.toml`}</span>
      <Select
        label="Does"
        value={trigger.kind}
        onChange={(event) => {
          const kind = event.target.value as TriggerKind;
          onChange({ kind, name: (kind === "command" ? MOCK_COMMANDS : MOCK_SKILLS)[0] });
        }}
      >
        <option value="command">Command in armada.yml</option>
        <option value="skill">Skill</option>
      </Select>
      <Select label={trigger.kind === "command" ? "Command" : "Skill"} value={trigger.name} onChange={(event) => onChange({ name: event.target.value })}>
        {held.map((one) => (
          <option key={one} value={one}>
            {one}
          </option>
        ))}
      </Select>
      <Fails block={trigger.block} repair={trigger.repair} onChange={onChange} />
      {onRemove === undefined ? null : (
        <div>
          <Button variant="ghost" onClick={onRemove}>
            <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove trigger
          </Button>
        </div>
      )}
    </>
  );
}

/** What fired in a Job: the result, or a repair Drone at work, and where each is set. */
export function FiredTriggers({ triggers }: { triggers: readonly FiredTrigger[] }) {
  return (
    <ul className="armada-triggers" aria-label="Triggers that fired">
      {triggers.map((one) => (
        <li key={`${one.name}|${one.when}`} className="armada-triggers__row armada-triggers__row--fired">
          <span className="armada-triggers__fired">
            {one.state === "passed" ? (
              <Tooltip label="Passed">
                <span className="armada-triggers__mark" data-hue="passed" role="img" aria-label="Passed">
                  <Check size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : (
              <>
                <Tooltip label="Failed">
                  <span className="armada-triggers__mark" data-hue="failed" role="img" aria-label="Failed">
                    <X size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <Tooltip label="Repair Drone working">
                  <span className="armada-triggers__mark" data-hue="running" data-pulsing="" role="img" aria-label="Repair Drone working">
                    <CircleDot size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </>
            )}
          </span>
          <span className="armada-triggers__name">{one.name}</span>
          <span className="armada-triggers__when">{one.when}</span>
          <PlaceMark place={one.place} />
        </li>
      ))}
    </ul>
  );
}

const KIND_SAID: Record<InsertedKind, string> = { script: "Script", skill: "Skill", drone: "Drone step" };

/** The `+` between steps: a menu of what a Job can be given. */
export function AddStep({ label, onPick }: { label: string; onPick: (kind: InsertedKind) => void }) {
  return (
    <span className="armada-triggers__add">
      <Tooltip label="Add a step to this Job only">
        <span>
          <DropdownMenu
            triggerLabel={label}
            icon={SquarePlus}
            portal
            entries={(Object.keys(KIND_SAID) as InsertedKind[]).map((id) => ({ kind: "item", id, label: KIND_SAID[id] }))}
            onSelect={(id) => onPick(id as InsertedKind)}
          />
        </span>
      </Tooltip>
    </span>
  );
}

/** The state an added step is in, as its card's activity. */
const ACTIVITY = { planned: "not_started", firing: "running", passed: "advanced" } as const;
const TOKEN = { planned: "--status-not-started", firing: "--status-running", passed: "--status-completed-success" } as const;

/** An added step as a card: its own band and the `webhook` mark, so it reads apart from the workflow's. */
export function insertedCard(one: Inserted, selected: boolean, onOpen: () => void): WorkflowStepCardProps {
  const fails = [one.block ? "blocks" : "", one.repair ? "repairs" : ""].filter((word) => word !== "").join(", ");
  return {
    kind: "step",
    name: one.kind === "drone" ? "Drone step" : one.name,
    ...(one.kind === "drone" ? {} : { nameIsAnIdentifier: true }),
    activity: ACTIVITY[one.state],
    mark: { icon: Webhook, token: TOKEN[one.state] },
    said: `Added to this Job only, ${one.state}`,
    added: true,
    facts: [{ value: KIND_SAID[one.kind].toLowerCase() }, ...(fails === "" ? [] : [{ value: fails }])],
    selected,
    onOpen,
  };
}

/** The fields of a step added to one Job, and the act that keeps it for every Job. */
export function InsertedFields({
  one,
  onChange,
  onKeep,
  onRemove,
}: {
  one: Inserted;
  onChange: (next: Partial<Inserted>) => void;
  onKeep: () => void;
  onRemove: () => void;
}) {
  const names = one.kind === "script" ? MOCK_COMMANDS : MOCK_SKILLS;
  return (
    <>
      <span className="armada-triggers__fact">{`${KIND_SAID[one.kind]}, this Job only`}</span>
      {one.kept === undefined ? null : <span className="armada-triggers__fact">{`Kept: ${PLACE[one.kept].said.toLowerCase()}`}</span>}
      {one.kind === "drone" ? (
        <Textarea label="Brief" rows={4} value={one.brief} onChange={(event) => onChange({ brief: event.target.value })} />
      ) : (
        <Select label={one.kind === "script" ? "Script" : "Skill"} value={one.name} onChange={(event) => onChange({ name: event.target.value })}>
          {names.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </Select>
      )}
      <Fails block={one.block} repair={one.repair} onChange={onChange} />
      <div className="armada-triggers__acts">
        {one.kind === "drone" ? (
          <Tooltip label="Triggers run a script or a skill">
            <span>
              <Button variant="secondary" disabled>
                Keep for every Job
              </Button>
            </span>
          </Tooltip>
        ) : (
          <Button variant="secondary" disabled={one.kept !== undefined} onClick={onKeep}>
            Keep for every Job
          </Button>
        )}
        <Button variant="ghost" onClick={onRemove}>
          <Trash2 size={12} strokeWidth={2} aria-hidden /> Remove step
        </Button>
      </div>
    </>
  );
}
