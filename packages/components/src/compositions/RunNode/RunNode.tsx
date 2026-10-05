import { Fragment } from "react";
import { Ellipsis, Eye, Scale, ShieldEllipsis, type LucideIcon } from "lucide-react";

import { CHECK_OUTCOME, CRITERION_VERDICT_JUDGE, STEP_STATE } from "../../generated/vocabulary";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

// One node of a Job's run on the approval canvas: a band naming its kind in
// its state's hue, then a body of three lines (the owner's worktree-slot card,
// 4 Oct 2026).

export type RunNodeKind =
  | "studio"
  | "brief"
  | "base"
  | "step"
  | "gate"
  | "stack"
  | "fan"
  | "group"
  | "task"
  | "more"
  | "job"
  | "done"
  | "pr"
  | "land";

/** A value on a node's face, named by its tooltip. `tuned` is one moved off its default. */
export type RunNodeTrait = { key: string; value: string; tuned?: boolean };

/** `ahead` is the gate, where nothing has run; `upcoming` is a running Job's node not reached yet. */
export type RunNodeState = "ahead" | "upcoming" | "live" | "done";

/** Where one command of a gate's Checks is. `off` is one this Job does not run. */
export type GateCommandOutcome = "passed" | "failed" | "running" | "waiting" | "off";

/** One judge of a panel: its verdict once it landed, `judging` while it reads, `pending` before. */
export type PanelMark = "met" | "not_met" | "judging" | "pending";

/**
 * A gate stage's face, one per kind of gate. Opt-in: a node with no `gate` is
 * the card it always was. State is the frame (`activity`, `state`), never a
 * field here.
 */
export type RunNodeGate =
  | {
      kind: "checks";
      commands: readonly { name: string; outcome: GateCommandOutcome }[];
      /** How long they have run, once running. */
      elapsed?: string;
      /** The last line a live command printed. */
      output?: string;
    }
  | {
      kind: "judge";
      /** One mark per judge, `panel_size` of them. */
      panel: readonly PanelMark[];
      /** A refusal's first line. */
      refusal?: string;
    }
  | {
      kind: "you";
      /** What it asks of the person. */
      asking?: string;
      /** How long it has waited. */
      waited?: string;
    };

export type RunNodeProps = {
  kind: RunNodeKind;
  /** The node's name, which it is read by. */
  name: string;
  /** The body's first line where it is not the name — the brief's title, the base's branch. */
  face?: string;
  /** A value a person copies — a branch — so mono. */
  faceMono?: boolean;
  /** Pressing it opens somewhere else: the title takes the accent. */
  links?: boolean;
  /** Its id, right-aligned in the band: a step's id, a group's. */
  id?: string;
  /** The body's second line, in words. Absent draws `traits` there. */
  line?: string;
  traits?: readonly RunNodeTrait[];
  meta?: readonly RunNodeTrait[];
  /** Done when's criteria, the body's lines; the first three are drawn. */
  items?: readonly string[];
  activity: StepActivity;
  /** The registry's word for the state, read to somebody who cannot see the mark. */
  said: string;
  /** A registry row's own glyph and token, in place of the step mark. */
  mark?: { icon: LucideIcon; token: string };
  state: RunNodeState;
  /** A gate stage's face, in place of the title and the lines. */
  gate?: RunNodeGate;
  /** Drawn at `--w-workflow-task-node`: a gate or a group beside or under the step it belongs to. */
  narrow?: boolean;
  selected?: boolean;
  /** Absent draws a node that is not a control. */
  onOpen?: () => void;
};

/** Every node is one card, one size: `RunNode.css` declares it off the tokens. A number because React Flow places by number. */
export const RUN_NODE_HEIGHT = 112;
export const RUN_NODE_WIDTH = 260;
export const RUN_NODE_NARROW = 196;
/** A gate stage is the lighter card between two steps: `RunNode.css` declares it off the tokens. */
/** A task hung under its group: the band and one title line, so a chain of four stays short. */
export const RUN_NODE_TASK_HEIGHT = 66;
export const RUN_NODE_GATE_HEIGHT = 44;
export const RUN_NODE_GATE_WIDTH = 216;

/** The band's word for each kind. A kind, never a state: the state is the band's hue and glyph. */
const KIND: Record<RunNodeKind, string> = {
  studio: "Studio",
  brief: "Brief",
  base: "Base",
  step: "Step",
  gate: "Checks",
  stack: "Plan",
  fan: "Jobs",
  group: "Group",
  task: "Task",
  more: "More",
  job: "Job",
  done: "Done when",
  pr: "Pull request",
  land: "Land",
};

/** What the band's hue says, off the step machine's own states. */
function toneOf(state: RunNodeState, activity: StepActivity): string {
  if (state === "ahead" && activity === "not_started") return "neutral";
  if (state === "upcoming") return "upcoming";
  if (activity === "awaiting_human") return "waiting";
  if (activity === "stopped") return "blocked";
  if (activity === "failed") return "failed";
  if (state === "done" || activity === "advanced") return "done";
  if (state === "live") return "live";
  return "neutral";
}

const OUTCOME_ICON = {
  passed: CHECK_OUTCOME["passed"]?.icon,
  failed: CHECK_OUTCOME["failed"]?.icon,
  running: STEP_STATE["running"]?.icon,
  waiting: STEP_STATE["not_started"]?.icon,
  off: CHECK_OUTCOME["skipped"]?.icon,
} as const;

const JUDGE_ICON = {
  met: CRITERION_VERDICT_JUDGE["met"]?.icon,
  not_met: CRITERION_VERDICT_JUDGE["not_met"]?.icon,
  judging: STEP_STATE["running"]?.icon,
  pending: STEP_STATE["not_started"]?.icon,
} as const;

const JUDGE_SAID = {
  met: CRITERION_VERDICT_JUDGE["met"]?.verb ?? "met",
  not_met: CRITERION_VERDICT_JUDGE["not_met"]?.verb ?? "not met",
  judging: STEP_STATE["running"]?.verb ?? "running",
  pending: STEP_STATE["not_started"]?.verb ?? "not started",
} as const;

/** The glyph for each kind of gate: the phase track's own. */
const GATE_GLYPH = { checks: ShieldEllipsis, judge: Scale, you: Eye } as const;

/**
 * A gate stage's one line, inside its capsule. A step is a card; a gate is a
 * valve in the line between two steps, so everything reads on one row.
 */
function GateLine({ gate, named, meta }: { gate: RunNodeGate; named: string; meta: readonly RunNodeTrait[] }) {
  const Kind = GATE_GLYPH[gate.kind];
  const tuned = meta.filter((one) => one.tuned === true);
  return (
    <>
      <span className="armada-run-node__capsule">
        <Tooltip label={named}>
          <span className="armada-run-node__kind-glyph">
            <Kind size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        {gate.kind === "checks" ? (
          <span className="armada-run-node__marks">
            {gate.commands
              .filter((one) => one.outcome !== "off")
              .map((one) => {
                const Icon = one.outcome === "waiting" ? STEP_STATE["not_started"]?.icon : OUTCOME_ICON[one.outcome];
                return (
                  <Tooltip key={one.name} label={`${one.name}, ${one.outcome}`}>
                    <span className="armada-run-node__mark" data-outcome={one.outcome}>
                      {Icon === undefined || Icon === null ? null : <Icon size={14} strokeWidth={2} aria-hidden />}
                      {one.outcome === "failed" ? <span className="armada-run-node__mark-name">{one.name}</span> : null}
                    </span>
                  </Tooltip>
                );
              })}
            {gate.elapsed === undefined ? null : (
              <Tooltip label="Running for">
                <span className="armada-run-node__elapsed">{gate.elapsed}</span>
              </Tooltip>
            )}
          </span>
        ) : gate.kind === "judge" ? (
          <span className="armada-run-node__marks">
            {gate.panel.map((mark, at) => {
              const Icon = JUDGE_ICON[mark];
              return (
                <Tooltip key={at} label={`Judge ${at + 1}, ${JUDGE_SAID[mark]}`}>
                  <span
                    className="armada-run-node__mark"
                    data-verdict={mark}
                    role="img"
                    aria-label={`Judge ${at + 1}, ${JUDGE_SAID[mark]}`}
                  >
                    {Icon === undefined || Icon === null ? null : <Icon size={16} strokeWidth={2} aria-hidden />}
                  </span>
                </Tooltip>
              );
            })}
            {gate.refusal === undefined ? null : <span className="armada-run-node__mark-name">{gate.refusal}</span>}
          </span>
        ) : (
          <span className="armada-run-node__marks">
            {gate.waited === undefined ? (
              <span className="armada-run-node__mark-name armada-run-node__mark-name--quiet">
                {gate.asking}
              </span>
            ) : (
              <Tooltip label="Waiting for you">
                <span className="armada-run-node__elapsed" data-held>
                  {gate.waited}
                </span>
              </Tooltip>
            )}
          </span>
        )}
        {tuned.length === 0 ? null : <Values of={tuned} className="armada-run-node__tuned" />}
      </span>
      {gate.kind === "checks" && gate.output !== undefined ? (
        <span className="armada-run-node__strip">{gate.output}</span>
      ) : null}
    </>
  );
}

function Values({ of, className }: { of: readonly RunNodeTrait[]; className: string }) {
  return (
    <span className={className}>
      {of.map((one, at) => (
        <Fragment key={one.key}>
          {at === 0 ? null : (
            <span className="armada-run-node__sep" aria-hidden>
              {" · "}
            </span>
          )}
          <Tooltip label={one.key}>
            <span className="armada-run-node__value" data-tuned={one.tuned || undefined}>
              {one.value}
            </span>
          </Tooltip>
        </Fragment>
      ))}
    </span>
  );
}

export function RunNode({
  kind,
  name,
  face,
  faceMono = false,
  links = false,
  id,
  line,
  traits = [],
  meta = [],
  items = [],
  activity,
  said,
  mark,
  gate,
  state,
  narrow = false,
  selected = false,
  onOpen,
}: RunNodeProps) {
  const named = `${name}, ${said}`;
  // Not made yet: Groups before a plan, Jobs before a wave. A faint outline and its word.
  const ghost = kind === "stack" || kind === "fan" || kind === "more";
  const attributes = {
    className: "armada-run-node",
    "data-kind": kind,
    "data-tone": ghost ? "ghost" : toneOf(state, activity),
    "data-narrow": narrow || undefined,
    "data-gate": gate?.kind,
  };
  const glyph =
    mark === undefined ? (
      <StepActivityMark activity={activity} label={said} says={named} />
    ) : (
      <Tooltip asChild label={named}>
        <span className="armada-step-mark">
          <mark.icon size={12} strokeWidth={2} aria-hidden />
          <span className="armada-step-mark__name">{said}</span>
        </span>
      </Tooltip>
    );
  const body = gate !== undefined ? (
    <GateLine gate={gate} named={named} meta={meta} />
  ) : kind === "more" ? (
    <Tooltip asChild label={said}>
      <Ellipsis size={16} aria-hidden />
    </Tooltip>
  ) : ghost ? (
    <>
      <span className="armada-run-node__ghost">{KIND[kind]}</span>
      {line === undefined ? null : <span className="armada-run-node__ghost-line">{line}</span>}
    </>
  ) : (
    <>
      <span className="armada-run-node__band">
        {glyph}
        <span className="armada-run-node__kind" aria-hidden>
          {KIND[kind]}
        </span>
        {id === undefined ? null : <span className="armada-run-node__id">{id}</span>}
      </span>
      <span className="armada-run-node__body">
        {/* A node with items is its items: the band already names it. */}
        {items.length > 0 ? null : (
          <span
            className="armada-run-node__title"
            data-mono={faceMono || undefined}
            data-links={links || undefined}
          >
            {face ?? name}
          </span>
        )}
        {items.length > 0 ? (
          items.slice(0, 3).map((item) => (
            <span key={item} className="armada-run-node__item">
              {item}
            </span>
          ))
        ) : (
          <>
            {line !== undefined ? (
              <span className="armada-run-node__line">{line}</span>
            ) : traits.length > 0 ? (
              <Values of={traits} className="armada-run-node__traits" />
            ) : null}
            {meta.length === 0 ? null : <Values of={meta} className="armada-run-node__meta" />}
          </>
        )}
      </span>
    </>
  );
  return onOpen === undefined ? (
    <span {...attributes} role="group" aria-label={named}>
      {body}
    </span>
  ) : (
    <button {...attributes} type="button" aria-current={selected ? "true" : undefined} aria-label={named} onClick={onOpen}>
      {body}
    </button>
  );
}
