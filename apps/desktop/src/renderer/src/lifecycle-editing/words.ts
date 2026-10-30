// A rule's If and Do as the editor's rows say them, from the structure Fleet serves. **Said from the definition and not from an instance**: the rule has not fired, so
// a ticket is "the ticket" and a state is named by the label its lifecycle gives it. Fleet's own line for an act that has fired (`saying.rs`) names the instance.

import type { LifecycleDefinition, LifecycleRule, RuleAct, RuleAmong, RuleCondition, RuleTarget } from "@armada/protocol";
import type { ActKind } from "@armada/components";

/** The longest a prompt or a message is quoted in a line. */
const QUOTED = 60;

const cut = (text: string): string => {
  const first = text.split("\n")[0]?.trim() ?? "";
  return first.length > QUOTED ? `${first.slice(0, QUOTED).trimEnd()}...` : first;
};

type Lifecycles = readonly Pick<LifecycleDefinition, "id" | "name" | "states">[];

/** The lifecycle a target is of: the one named, or the one whose entering set the rule off. */
export function laneOf(among: RuleAmong, rule: Pick<LifecycleRule, "when">): string | undefined {
  return among.kind === "entered" ? (rule.when.kind === "enters" ? rule.when.lifecycle : undefined) : among.lifecycle;
}

/** What a lifecycle's instances are called in a line: a ticket, a pull request, a deploy. */
const noun = (lifecycle: string | undefined, lifecycles: Lifecycles): string => {
  const name = lifecycles.find((one) => one.id === lifecycle)?.name ?? lifecycle ?? "instance";
  return name.toLowerCase();
};

const stateLabel = (lifecycle: string | undefined, state: string, lifecycles: Lifecycles): string => lifecycles.find((one) => one.id === lifecycle)?.states.find((each) => each.id === state)?.label ?? state;

function thing(target: RuleTarget, rule: Pick<LifecycleRule, "when">, lifecycles: Lifecycles): string {
  const lane = laneOf(target.among, rule);
  const word = noun(lane, lifecycles);
  const where = target.in_state === undefined ? "" : ` in ${stateLabel(lane, target.in_state, lifecycles)}`;
  return `the ${word}${where}`;
}

/** The kind of act a line is, which the edge draws a glyph for. */
export function kindOf(act: RuleAct): ActKind {
  switch (act.act) {
    case "tracker_assign":
    case "tracker_move":
    case "tracker_close":
      return "tracker";
    case "forge_request_reviewers":
    case "forge_mark_ready":
    case "forge_merge":
    case "forge_auto_merge":
      return "forge";
    case "draft":
      return "message";
    case "start_job":
      return "job";
    case "start_watch":
      return "watch";
    case "tell":
    case "wake":
      return "tell";
    case "run_command":
    case "run_skill":
      return "command";
    case "run_drone":
      return "agent";
  }
}

export function sayAct(act: RuleAct, rule: Pick<LifecycleRule, "when">, lifecycles: Lifecycles): string {
  switch (act.act) {
    case "tracker_assign":
      return `Assign ${thing(act.target, rule, lifecycles)} to ${act.to}`;
    case "tracker_move":
      return `Move ${thing(act.target, rule, lifecycles)} to ${stateLabel(laneOf(act.target.among, rule), act.to, lifecycles)}`;
    case "tracker_close":
      return `Close ${thing(act.target, rule, lifecycles)}`;
    case "forge_request_reviewers":
      return `Request review on ${thing(act.target, rule, lifecycles)} from ${act.from.join(", ")}`;
    case "forge_mark_ready":
      return `Mark ${thing(act.target, rule, lifecycles)} ready`;
    case "forge_merge":
      return `Merge ${thing(act.target, rule, lifecycles)}`;
    case "forge_auto_merge":
      return `Turn on auto-merge for ${thing(act.target, rule, lifecycles)}`;
    case "draft":
      return `Draft a message on ${thing(act.target, rule, lifecycles)}: ${cut(act.template)}`;
    case "start_job":
      return `Start a Job: ${cut(act.prompt)}`;
    case "start_watch":
      return act.deploy === undefined || act.deploy === "" ? `Start the ${act.recipe} watch` : `Start the ${act.deploy} deploy watch`;
    case "tell":
      return `Tell the agent: ${cut(act.message)}`;
    case "wake":
      return "Wake the agent";
    case "run_command":
      return `Run ${act.command}`;
    case "run_skill":
      return `Run the ${act.skill} skill`;
    case "run_drone":
      return `Start a Drone: ${cut(act.prompt)}`;
  }
}

const QUANTITY: Readonly<Record<string, string>> = { every: "Every", any: "Any", no: "No" };

function among(one: RuleAmong, rule: Pick<LifecycleRule, "when">, lifecycles: Lifecycles): string {
  switch (one.kind) {
    case "entered":
      return `the ${noun(laneOf(one, rule), lifecycles)}`;
    case "linked_from":
      return `${noun(one.lifecycle, lifecycles)} it came from`;
    case "linking_to":
      return `${noun(one.lifecycle, lifecycles)} that came from it`;
    case "held":
      return `${noun(one.lifecycle, lifecycles)} the Job holds`;
  }
}

export function sayCondition(condition: RuleCondition, rule: Pick<LifecycleRule, "when">, lifecycles: Lifecycles): string {
  switch (condition.kind) {
    case "instances": {
      const lane = laneOf(condition.among, rule);
      return `${QUANTITY[condition.quantity] ?? condition.quantity} ${among(condition.among, rule, lifecycles)} is ${stateLabel(lane, condition.in_state, lifecycles)}`;
    }
    case "workflow":
      return `The Job follows ${condition.workflow}`;
    case "at_step":
      return `The Job is on ${condition.step}`;
    case "pull_request_opened":
      return "The Job has opened a pull request";
  }
}
