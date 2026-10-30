import { X } from "lucide-react";
import { useId, useState } from "react";
import type { ReactNode } from "react";
import type { LifecycleRule, RuleAct, RuleAmong, RuleCondition, RuleDoing, RuleTarget, RuleWhen } from "@armada/protocol";

import { Button } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { Autonomy, LifecycleKind } from "../LaneCanvas/model";
import { Field, Lines, Wrapped } from "./InspectorFields";

/**
 * A rule's fields as pickers over what Fleet can run, so every change is one it keeps: when it fires (a state entered, or a step starting or ending), what it checks (`core_model::rule::Select`'s closed set),
 * what it does (`Act`'s kinds, each with its target and state), how far it goes alone and what it does when an act fails. It draws the rows of a rule's panel and hands back the whole piece.
 *
 * **Nothing is offered that Fleet would refuse.** An instance that entered, or those linked to it, is offered only to a rule that fires on a state entered; changing the moment to a step turns them into the ones the Job
 * holds. A ticket is moved only to a state of its own lifecycle, a pull request is merged and a ticket is closed only through the forge's and the tracker's lifecycles. An act that needs words, such as what to tell the agent,
 * stays in the form until they are typed and is added then. **A rule lands on a node of the lanes** (a state it moves a ticket to, or the step the Job is on), so one that would land on none gets the step it fires on.
 */
export type RuleFormLifecycle = { id: string; name: string; kind: LifecycleKind | undefined; states: readonly { id: string; label: string }[] };

const noun = (name: string): string => name.toLowerCase();

/** The word an act is added by, and the kind of act it is for the autonomy default. */
export const ACTS: readonly { act: RuleAct["act"]; said: string; kind: string }[] = [
  { act: "tracker_assign", said: "Assign a ticket", kind: "tracker_write" },
  { act: "tracker_move", said: "Move a ticket", kind: "tracker_write" },
  { act: "tracker_close", said: "Close a ticket", kind: "tracker_write" },
  { act: "forge_request_reviewers", said: "Request review", kind: "forge_write" },
  { act: "forge_mark_ready", said: "Mark a pull request ready", kind: "forge_write" },
  { act: "forge_merge", said: "Merge a pull request", kind: "forge_write" },
  { act: "forge_auto_merge", said: "Turn on auto-merge", kind: "forge_write" },
  { act: "draft", said: "Draft a message", kind: "draft" },
  { act: "start_job", said: "Start a Job", kind: "start_job" },
  { act: "start_watch", said: "Start a watch", kind: "start_watch" },
  { act: "tell", said: "Tell the agent", kind: "tell_agent" },
  { act: "wake", said: "Wake the agent", kind: "tell_agent" },
  { act: "run_command", said: "Run a command", kind: "run_command" },
  { act: "run_skill", said: "Run a skill", kind: "run_agent" },
  { act: "run_drone", said: "Start a Drone", kind: "run_agent" },
];

const said = (act: RuleAct["act"]): string => ACTS.find((one) => one.act === act)?.said ?? act;

/** The kind of lifecycle an act is done to: the tracker's tickets, the forge's pull requests. */
const TARGETS: Readonly<Partial<Record<RuleAct["act"], readonly (LifecycleKind | undefined)[]>>> = {
  tracker_assign: ["tracker", undefined],
  tracker_move: ["tracker", undefined],
  tracker_close: ["tracker", undefined],
  forge_request_reviewers: ["forge"],
  forge_mark_ready: ["forge"],
  forge_merge: ["forge"],
  forge_auto_merge: ["forge"],
  draft: ["tracker", undefined, "forge"],
};

/** What an act cannot be kept without: words, typed. */
function missing(act: RuleAct): boolean {
  switch (act.act) {
    case "tracker_assign":
      return act.to.trim() === "";
    case "forge_request_reviewers":
      return act.from.length === 0;
    case "draft":
      return act.template.trim() === "";
    case "start_job":
    case "run_drone":
      return act.prompt.trim() === "";
    case "start_watch":
      return act.recipe.trim() === "";
    case "tell":
      return act.message.trim() === "";
    case "run_command":
      return act.command.trim() === "";
    case "run_skill":
      return act.skill.trim() === "";
    default:
      return false;
  }
}

/** The strongest word among a rule's acts: any that asks makes it Asks, and all of them off make it Off. */
export function autonomyOfWire(rule: Pick<LifecycleRule, "autonomy" | "does">): Autonomy {
  const words = rule.autonomy === undefined ? rule.does.map((one) => one.autonomy) : [rule.autonomy];
  if (words.includes("asks")) return "asks";
  return words.length > 0 && words.every((one) => one === "off") ? "off" : "acts";
}

const AUTONOMY: readonly { value: Autonomy; said: string }[] = [
  { value: "acts", said: "Acts" },
  { value: "asks", said: "Asks" },
  { value: "off", said: "Off" },
];

/** The most a failed act is tried again, which Fleet caps. */
const RETRIES = [0, 1, 2, 3, 4, 5] as const;

type Choice = { value: string; label: string };

/** A dropdown that names itself. One whose value the list does not hold opens on nothing, so it never shows a guess. */
function Pick({ name, value, items, blank = false, onChange }: { name: string; value: string; items: readonly Choice[]; blank?: boolean | string; onChange: (value: string) => void }) {
  const open = value === "" || !items.some((one) => one.value === value);
  return (
    <Select aria-label={name} value={open ? "" : value} onChange={(event) => onChange(event.target.value)}>
      {open || blank !== false ? <option value="">{typeof blank === "string" ? blank : ""}</option> : null}
      {items.map((one) => (
        <option key={one.value} value={one.value}>
          {one.label}
        </option>
      ))}
    </Select>
  );
}

const amongValue = (among: RuleAmong): string => (among.kind === "entered" ? "entered" : `${among.kind}:${among.lifecycle}`);
const amongOf = (value: string): RuleAmong => {
  const [kind, lifecycle = ""] = value.split(":");
  return kind === "entered" ? { kind: "entered" } : ({ kind, lifecycle } as RuleAmong);
};

/** The lifecycle a set of instances is of, given the one whose entering set the rule off. */
const lifecycleOf = (among: RuleAmong, when: RuleWhen): string | undefined => (among.kind === "entered" ? (when.kind === "enters" ? when.lifecycle : undefined) : among.lifecycle);

const whenValue = (when: RuleWhen): string => (when.kind === "enters" ? `enters|${when.lifecycle}|${when.state}` : when.kind === "step_starts" ? `starts|${when.step}` : when.kind === "step_ends" ? `ends|${when.step}` : "");
const whenOf = (value: string): RuleWhen | undefined => {
  const [kind, a = "", b = ""] = value.split("|");
  if (kind === "enters") return { kind: "enters", lifecycle: a, state: b };
  if (kind === "starts") return { kind: "step_starts", step: a };
  return kind === "ends" ? { kind: "step_ends", step: a } : undefined;
};

/** What the instances of a set are called in a row: `ticket that entered`, `pull request it came from`. **`bare` leaves the lifecycle out**, where the act before it names the thing: `Move a ticket`, `the Job holds`. */
function amongSaid(among: RuleAmong, lifecycles: readonly RuleFormLifecycle[], when: RuleWhen, bare = false): string {
  const name = bare ? "" : `${noun(lifecycles.find((one) => one.id === lifecycleOf(among, when))?.name ?? lifecycleOf(among, when) ?? "instance")} `;
  return among.kind === "entered" ? `${name}that entered` : among.kind === "linked_from" ? `${name}it came from` : among.kind === "linking_to" ? `${name}that came from it` : `${name}the Job holds`;
}

/** An instance that entered, and those linked to it, mean something only to a rule that fires on a state entered: with the moment changed they become the ones the Job holds. */
function held(among: RuleAmong, was: RuleWhen, now: RuleWhen): RuleAmong {
  if (among.kind === "held") return among;
  if (among.kind === "entered") {
    const lifecycle = was.kind === "enters" ? was.lifecycle : undefined;
    return now.kind === "enters" && now.lifecycle === lifecycle ? among : lifecycle === undefined ? among : { kind: "held", lifecycle };
  }
  return now.kind === "enters" ? among : { kind: "held", lifecycle: among.lifecycle };
}

const retarget = (target: RuleTarget, was: RuleWhen, now: RuleWhen): RuleTarget => ({ ...target, among: held(target.among, was, now) });

function withWhen(rule: LifecycleRule, now: RuleWhen): LifecycleRule {
  return {
    ...rule,
    when: now,
    conditions: rule.conditions.map((one) => (one.kind === "instances" ? { ...one, among: held(one.among, rule.when, now) } : one)),
    does: rule.does.map((one) => ("target" in one.what ? { ...one, what: { ...one.what, target: retarget(one.what.target, rule.when, now) } as RuleAct } : one)),
  };
}

/** Where a target may be: the instances that entered where the rule fires on one, what is linked to it, and what the Job holds, of the lifecycles the act is done to. */
function instancesFor(kinds: readonly (LifecycleKind | undefined)[] | undefined, lifecycles: readonly RuleFormLifecycle[], when: RuleWhen): Choice[] {
  const ours = lifecycles.filter((one) => kinds === undefined || kinds.includes(one.kind));
  // Where an act names the thing and one lifecycle is of that kind, the lifecycle is not said again.
  const bare = kinds !== undefined && ours.length === 1;
  const entered = when.kind === "enters" ? ours.filter((one) => one.id === when.lifecycle) : [];
  const linked = when.kind === "enters" ? ours.flatMap((one): RuleAmong[] => [{ kind: "linked_from", lifecycle: one.id }, { kind: "linking_to", lifecycle: one.id }]) : [];
  return [...entered.map((): RuleAmong => ({ kind: "entered" })), ...linked, ...ours.map((one): RuleAmong => ({ kind: "held", lifecycle: one.id }))].map((one) => ({ value: amongValue(one), label: amongSaid(one, lifecycles, when, bare) }));
}

const targetsFor = (act: RuleAct["act"], lifecycles: readonly RuleFormLifecycle[], when: RuleWhen): Choice[] => instancesFor(TARGETS[act], lifecycles, when);

/** The conditions on instances, which look at any lifecycle. */
const amongsFor = (lifecycles: readonly RuleFormLifecycle[], when: RuleWhen): Choice[] => instancesFor(undefined, lifecycles, when);

/** An act as it first stands once added: aimed at the first place it can be, with the first state to move to. */
function started(act: RuleAct["act"], lifecycles: readonly RuleFormLifecycle[], when: RuleWhen): RuleAct | undefined {
  const target = (): RuleTarget | undefined => {
    const first = targetsFor(act, lifecycles, when)[0];
    return first === undefined ? undefined : { among: amongOf(first.value) };
  };
  const into = (target: RuleTarget): string => lifecycles.find((one) => one.id === lifecycleOf(target.among, when))?.states[0]?.id ?? "";
  const aimed = TARGETS[act] !== undefined ? target() : undefined;
  if (TARGETS[act] !== undefined && aimed === undefined) return undefined;
  switch (act) {
    case "tracker_assign":
      return { act, to: "", target: aimed! };
    case "tracker_move":
      return { act, to: into(aimed!), target: aimed! };
    case "tracker_close":
    case "forge_mark_ready":
    case "forge_merge":
    case "forge_auto_merge":
      return { act, target: aimed! };
    case "forge_request_reviewers":
      return { act, from: [], target: aimed! };
    case "draft":
      return { act, template: "", target: aimed! };
    case "start_job":
      return { act, prompt: "" };
    case "start_watch":
      return { act, recipe: "", values: {} };
    case "tell":
      return { act, message: "" };
    case "wake":
      return { act };
    case "run_command":
      return { act, command: "" };
    case "run_skill":
      return { act, skill: "" };
    case "run_drone":
      return { act, prompt: "" };
  }
}

const startedCondition = (kind: string, lifecycles: readonly RuleFormLifecycle[], steps: readonly { id: string }[], workflows: readonly string[], when: RuleWhen): RuleCondition | undefined => {
  if (kind === "pull_request_opened") return { kind };
  if (kind === "at_step") return steps[0] === undefined ? undefined : { kind, step: steps[0].id };
  if (kind === "workflow") return workflows[0] === undefined ? undefined : { kind, workflow: workflows[0] };
  const among = amongsFor(lifecycles, when)[0];
  const lifecycle = among === undefined ? undefined : lifecycles.find((one) => one.id === lifecycleOf(amongOf(among.value), when));
  return among === undefined || lifecycle?.states[0] === undefined ? undefined : { kind: "instances", among: amongOf(among.value), quantity: kind, in_state: lifecycle.states[0].id };
};

/** A row's controls side by side, with the words between them that make them read as one clause. */
function Clause({ children, remove }: { children: ReactNode; remove?: { name: string; onPress: () => void } | undefined }) {
  return (
    <div className="armada-wm-clause">
      {children}
      {remove === undefined ? null : (
        <Tooltip label={remove.name}>
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={remove.name} onClick={remove.onPress}>
            <X size={16} strokeWidth={2} aria-hidden="true" />
          </Button>
        </Tooltip>
      )}
    </div>
  );
}

const Word = ({ children }: { children: string }) => <span className="armada-wm-clause__word">{children}</span>;

/** A word with the control it leads, held together so a line breaks before the word and never between it and its control. */
const Led = ({ word, children }: { word: string; children: ReactNode }) => (
  <span className="armada-wm-clause__led">
    <Word>{word}</Word>
    {children}
  </span>
);

export type RuleFormProps = {
  /** The piece as this tab runs it. */
  rule: LifecycleRule;
  /** The lifecycles as this tab has them. */
  lifecycles: readonly RuleFormLifecycle[];
  /** The steps of the workflow as this tab runs it. */
  steps: readonly { id: string; label: string }[];
  /** Every workflow, by id. */
  workflows: readonly string[];
  /** Whether a rule lands on a node of the lanes, which is how it is drawn and picked. */
  lands: (rule: LifecycleRule) => boolean;
  /** The piece, changed. Only a piece Fleet keeps is handed back. */
  onChange: (rule: LifecycleRule) => void;
};

/** The rows of a rule's panel: the `dt` and `dd` of a definition list. */
export function RuleForm({ rule, lifecycles, steps, workflows, lands, onChange }: RuleFormProps) {
  const id = useId();
  const autonomy = autonomyOfWire(rule);
  /** An act that is waiting on its words. */
  const [making, setMaking] = useState<{ act: RuleAct; key: string } | undefined>(undefined);
  const gone = lifecycles.length === 0;

  /** The piece to keep: with a node to land on, which the step it fires on is where it has none. */
  const settle = (next: LifecycleRule): LifecycleRule => {
    if (next.when.kind === "job" || lands(next)) return next;
    const step = next.when.kind === "step_starts" || next.when.kind === "step_ends" ? next.when.step : steps[0]?.id;
    if (step === undefined) return next;
    return { ...next, conditions: [...next.conditions.filter((one) => one.kind !== "at_step"), { kind: "at_step", step }] };
  };
  const keep = (next: LifecycleRule) => onChange(settle(next));
  const doing = (what: RuleAct): RuleDoing => ({ what, kind: ACTS.find((one) => one.act === what.act)?.kind ?? "", autonomy });
  const setAct = (at: number, what: RuleAct) => keep({ ...rule, does: rule.does.map((one, index) => (index === at ? { ...one, what } : one)) });
  const setCondition = (at: number, next: RuleCondition) => keep({ ...rule, conditions: rule.conditions.map((one, index) => (index === at ? next : one)) });
  const canDropStep = lands({ ...rule, conditions: rule.conditions.filter((one) => one.kind !== "at_step") });

  const stateItems = (lifecycle: string | undefined): Choice[] => (lifecycles.find((one) => one.id === lifecycle)?.states ?? []).map((one) => ({ value: one.id, label: one.label }));
  const steppedItems: Choice[] = steps.map((one) => ({ value: one.id, label: one.label }));

  const whenGroups = [
    { label: "Steps", items: steps.flatMap((one) => [{ value: `starts|${one.id}`, label: `${one.label} starts` }, { value: `ends|${one.id}`, label: `${one.label} ends` }]) },
    ...lifecycles.map((one) => ({ label: one.name, items: one.states.map((state) => ({ value: `enters|${one.id}|${state.id}`, label: `${one.name} enters ${state.label}` })) })),
  ];
  const whenHeld = whenValue(rule.when);
  const whenOpen = !whenGroups.some((group) => group.items.some((one) => one.value === whenHeld));

  /** A target: which instances, and in which state. Drawn by calling it, so the fields in it keep what is typed as the form changes. */
  const aim = ({ act, target, onTarget, label, lead }: { act: RuleAct["act"]; target: RuleTarget; onTarget: (next: RuleTarget) => void; label: string; lead: string }) => {
    const lifecycle = lifecycleOf(target.among, rule.when);
    return (
      <>
        <Led word={lead}>
          <Pick name={`${label} on`} value={amongValue(target.among)} items={targetsFor(act, lifecycles, rule.when)} onChange={(value) => onTarget({ among: amongOf(value) })} />
        </Led>
        <Led word="in">
          <Pick name={`${label} state`} value={target.in_state ?? ""} blank="any state" items={stateItems(lifecycle)} onChange={(value) => onTarget({ among: target.among, ...(value === "" ? {} : { in_state: value }) })} />
        </Led>
      </>
    );
  };

  /** The fields of one act, which change it through `change`. */
  const fields = ({ what, change, name, lead }: { what: RuleAct; change: (next: RuleAct) => void; name: string; lead: string }) => {
    const key = `${id}-${name}`;
    switch (what.act) {
      case "tracker_assign":
        return (
          <>
            {aim({ act: what.act, target: what.target, label: name, lead, onTarget: (target) => change({ ...what, target }) })}
            <Field label="Assignee" value={what.to} onCommit={(to) => change({ ...what, to })} />
          </>
        );
      case "tracker_move": {
        const lifecycle = lifecycleOf(what.target.among, rule.when);
        return (
          <>
            {aim({ act: what.act, target: what.target, label: name, lead, onTarget: (target) => change({ ...what, target, to: lifecycleOf(target.among, rule.when) === lifecycle ? what.to : (stateItems(lifecycleOf(target.among, rule.when))[0]?.value ?? "") }) })}
            <Led word="to">
              <Pick name={`${name} to`} value={what.to} items={stateItems(lifecycle)} onChange={(to) => change({ ...what, to })} />
            </Led>
          </>
        );
      }
      case "tracker_close":
      case "forge_mark_ready":
      case "forge_merge":
      case "forge_auto_merge":
        return aim({ act: what.act, target: what.target, label: name, lead, onTarget: (target) => change({ ...what, target }) });
      case "forge_request_reviewers":
        return (
          <>
            {aim({ act: what.act, target: what.target, label: name, lead, onTarget: (target) => change({ ...what, target }) })}
            <Field label="Reviewers, comma separated" value={what.from.join(", ")} onCommit={(text) => change({ ...what, from: text.split(",").map((one) => one.trim()).filter((one) => one !== "") })} />
          </>
        );
      case "draft":
        return (
          <>
            {aim({ act: what.act, target: what.target, label: name, lead, onTarget: (target) => change({ ...what, target }) })}
            <Wrapped label="Message" keep={`workflows:draft:${key}`} value={what.template} onCommit={(template) => change({ ...what, template })} />
          </>
        );
      case "start_job":
        return (
          <>
            <Word>{lead}</Word>
            <Pick name={`${name} workflow`} value={what.workflow ?? ""} blank items={workflows.map((one) => ({ value: one, label: one }))} onChange={(workflow) => change({ act: what.act, prompt: what.prompt, ...(workflow === "" ? {} : { workflow }) })} />
            <Wrapped label="Prompt" keep={`workflows:prompt:${key}`} value={what.prompt} onCommit={(prompt) => change({ ...what, prompt })} />
          </>
        );
      case "start_watch":
        return (
          <>
            <Word>{lead}</Word>
            <Field label="Recipe" value={what.recipe} onCommit={(recipe) => change({ ...what, recipe })} />
            <Lines label="Values, name=value" keep={`workflows:values:${key}`} lines={Object.entries(what.values).map(([name, value]) => `${name}=${value}`)} onCommit={(lines) => change({ ...what, values: Object.fromEntries(lines.flatMap((line) => (line.includes("=") ? [[line.slice(0, line.indexOf("=")).trim(), line.slice(line.indexOf("=") + 1).trim()] as const] : []))) })} />
            <Field label="Deploy" value={what.deploy ?? ""} onCommit={(deploy) => change({ act: what.act, recipe: what.recipe, values: what.values, ...(deploy === "" ? {} : { deploy }) })} />
          </>
        );
      case "tell":
        return (
          <>
            <Word>{lead}</Word>
            <Wrapped label="Message" keep={`workflows:tell:${key}`} value={what.message} onCommit={(message) => change({ ...what, message })} />
          </>
        );
      case "wake":
        return <Word>{lead}</Word>;
      case "run_command":
        return (
          <>
            <Word>{lead}</Word>
            <Field label="Command" mono value={what.command} onCommit={(command) => change({ ...what, command })} />
          </>
        );
      case "run_skill":
        return (
          <>
            <Word>{lead}</Word>
            <Field label="Skill" value={what.skill} onCommit={(skill) => change({ ...what, skill })} />
          </>
        );
      case "run_drone":
        return (
          <>
            <Word>{lead}</Word>
            <Wrapped label="Prompt" keep={`workflows:drone:${key}`} value={what.prompt} onCommit={(prompt) => change({ ...what, prompt })} />
          </>
        );
    }
  };

  /** An act that is waiting on its words is added once it has them. */
  const completed = (what: RuleAct) => {
    if (missing(what)) return setMaking((was) => (was === undefined ? was : { ...was, act: what }));
    setMaking(undefined);
    keep({ ...rule, does: [...rule.does, doing(what)] });
  };

  const addable = ACTS.filter((one) => started(one.act, lifecycles, rule.when) !== undefined);
  const addable_conditions: { value: string; label: string }[] = [
    ...(amongsFor(lifecycles, rule.when).length === 0 ? [] : [{ value: "every", label: "Every" }, { value: "any", label: "Any" }, { value: "no", label: "No" }]),
    ...(steps.length === 0 || rule.conditions.some((one) => one.kind === "at_step") ? [] : [{ value: "at_step", label: "The Job is on" }]),
    ...(workflows.length === 0 || rule.conditions.some((one) => one.kind === "workflow") ? [] : [{ value: "workflow", label: "The Job follows" }]),
    ...(rule.conditions.some((one) => one.kind === "pull_request_opened") ? [] : [{ value: "pull_request_opened", label: "The Job has opened a pull request" }]),
  ];

  return (
    <>
      <dt id={`${id}-name`}>Name</dt>
      <dd>
        <Field labelledBy={`${id}-name`} value={rule.name} onCommit={(name) => name !== "" && keep({ ...rule, name })} />
      </dd>

      <dt id={`${id}-when`}>When</dt>
      <dd>
        <Select
          aria-labelledby={`${id}-when`}
          value={whenOpen ? "" : whenHeld}
          onChange={(event) => {
            const next = whenOf(event.target.value);
            if (next !== undefined) keep(withWhen(rule, next));
          }}
        >
          {whenOpen ? <option value="" /> : null}
          {whenGroups.map((group) => (
            <optgroup key={group.label} label={group.label}>
              {group.items.map((one) => (
                <option key={one.value} value={one.value}>
                  {one.label}
                </option>
              ))}
            </optgroup>
          ))}
        </Select>
      </dd>

      <dt id={`${id}-if`}>If</dt>
      <dd>
        <ul className="armada-wm-clauses" aria-labelledby={`${id}-if`}>
          {rule.conditions.map((one, at) => {
            // The workflow a rule belongs to is held by the rule's own tab as a condition like any other.
            const remove = one.kind === "at_step" && !canDropStep ? undefined : { name: "Remove condition", onPress: () => keep({ ...rule, conditions: rule.conditions.filter((_, index) => index !== at) }) };
            return (
              <li key={`${one.kind}-${at}`}>
                <Clause remove={remove}>
                  {one.kind === "instances" ? (
                    <>
                      <Pick name="Quantity" value={one.quantity} items={[{ value: "every", label: "Every" }, { value: "any", label: "Any" }, { value: "no", label: "No" }]} onChange={(quantity) => setCondition(at, { ...one, quantity })} />
                      <Pick
                        name="Instances"
                        value={amongValue(one.among)}
                        items={amongsFor(lifecycles, rule.when)}
                        onChange={(value) => {
                          const among = amongOf(value);
                          const lifecycle = lifecycleOf(among, rule.when);
                          const states = stateItems(lifecycle);
                          setCondition(at, { ...one, among, in_state: states.some((state) => state.value === one.in_state) ? one.in_state : (states[0]?.value ?? "") });
                        }}
                      />
                      <Word>is</Word>
                      <Pick name="State" value={one.in_state} items={stateItems(lifecycleOf(one.among, rule.when))} onChange={(in_state) => setCondition(at, { ...one, in_state })} />
                    </>
                  ) : one.kind === "at_step" ? (
                    <>
                      <Word>The Job is on</Word>
                      <Pick name="Step" value={one.step} items={steppedItems} onChange={(step) => setCondition(at, { kind: "at_step", step })} />
                    </>
                  ) : one.kind === "workflow" ? (
                    <>
                      <Word>The Job follows</Word>
                      <Pick name="Workflow" value={one.workflow} items={workflows.map((flow) => ({ value: flow, label: flow }))} onChange={(workflow) => setCondition(at, { kind: "workflow", workflow })} />
                    </>
                  ) : (
                    <Word>The Job has opened a pull request</Word>
                  )}
                </Clause>
              </li>
            );
          })}
        </ul>
        {addable_conditions.length === 0 || gone ? null : (
          <Pick
            name="Add a condition"
            value=""
            blank="Add condition"
            items={addable_conditions}
            onChange={(kind) => {
              const next = startedCondition(kind, lifecycles, steps, workflows, rule.when);
              if (next !== undefined) keep({ ...rule, conditions: [...rule.conditions, next] });
            }}
          />
        )}
      </dd>

      <dt id={`${id}-do`}>Do</dt>
      <dd>
        <ul className="armada-wm-clauses" aria-labelledby={`${id}-do`}>
          {rule.does.map((one, at) => (
            <li key={`${one.what.act}-${at}`}>
              <Clause remove={rule.does.length === 1 ? undefined : { name: "Remove act", onPress: () => keep({ ...rule, does: rule.does.filter((_, index) => index !== at) }) }}>
                {fields({ what: one.what, name: `${said(one.what.act)} ${at + 1}`, lead: said(one.what.act), change: (next) => !missing(next) && setAct(at, next) })}
              </Clause>
            </li>
          ))}
          {making === undefined ? null : (
            <li key={making.key}>
              <Clause remove={{ name: "Remove act", onPress: () => setMaking(undefined) }}>
                {fields({ what: making.act, name: said(making.act.act), lead: said(making.act.act), change: completed })}
              </Clause>
            </li>
          )}
        </ul>
        {making !== undefined || addable.length === 0 ? null : (
          <Pick
            name="Add an act"
            value=""
            blank="Add act"
            items={addable.map((one) => ({ value: one.act, label: one.said }))}
            onChange={(act) => {
              const next = started(act as RuleAct["act"], lifecycles, rule.when);
              if (next === undefined) return;
              if (missing(next)) setMaking({ act: next, key: `${act}-${rule.does.length}` });
              else keep({ ...rule, does: [...rule.does, doing(next)] });
            }}
          />
        )}
      </dd>

      <dt id={`${id}-autonomy`}>Autonomy</dt>
      <dd>
        <RadioGroup labelledBy={`${id}-autonomy`}>
          {AUTONOMY.map((one) => (
            <Radio key={one.value} name={`autonomy:${rule.id}`} checked={autonomy === one.value} onChange={() => keep({ ...rule, autonomy: one.value, does: rule.does.map((each) => ({ ...each, autonomy: one.value })) })}>
              {one.said}
            </Radio>
          ))}
        </RadioGroup>
      </dd>

      <dt id={`${id}-failure`}>On failure</dt>
      <dd>
        <div className="armada-wm-clause" role="group" aria-labelledby={`${id}-failure`}>
          <Pick
            name="Retries"
            value={String(rule.on_failure.retries)}
            items={RETRIES.map((one) => ({ value: String(one), label: one === 0 ? "No retries" : one === 1 ? "1 retry" : `${one} retries` }))}
            onChange={(retries) => keep({ ...rule, on_failure: { ...rule.on_failure, retries: Number(retries) } })}
          />
          <Pick
            name="Then"
            value={rule.on_failure.then}
            items={[
              { value: "tell_agent", label: "Then tell the agent" },
              { value: "ask_owner", label: "Then ask you" },
            ]}
            onChange={(then) => keep({ ...rule, on_failure: { ...rule.on_failure, then } })}
          />
        </div>
      </dd>
    </>
  );
}
