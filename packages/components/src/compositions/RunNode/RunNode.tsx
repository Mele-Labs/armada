import { Fragment } from "react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

// One node of a Job's run on the approval canvas: a band naming its kind in
// its state's hue, then a body of three lines (the owner's worktree-slot card,
// 4 Oct 2026). Prototype: no story until he has walked it.

export type RunNodeKind =
  | "brief"
  | "base"
  | "start"
  | "step"
  | "gate"
  | "stack"
  | "fan"
  | "group"
  | "job"
  | "done"
  | "pr"
  | "land";

/** A value on a node's face, named by its tooltip. `tuned` is one moved off its default. */
export type RunNodeTrait = { key: string; value: string; tuned?: boolean };

/** `ahead` is the gate, where nothing has run; `upcoming` is a running Job's node not reached yet. */
export type RunNodeState = "ahead" | "upcoming" | "live" | "done";

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

/** The band's word for each kind. A kind, never a state: the state is the band's hue and glyph. */
const KIND: Record<RunNodeKind, string> = {
  brief: "Brief",
  base: "Base",
  start: "Start",
  step: "Step",
  gate: "Checks",
  stack: "Groups",
  fan: "Jobs",
  group: "Group",
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
  state,
  narrow = false,
  selected = false,
  onOpen,
}: RunNodeProps) {
  const named = `${name}, ${said}`;
  // Not made yet: Groups before a plan, Jobs before a wave. A faint outline and its word.
  const ghost = kind === "stack" || kind === "fan";
  const attributes = {
    className: "armada-run-node",
    "data-kind": kind,
    "data-tone": ghost ? "ghost" : toneOf(state, activity),
    "data-narrow": narrow || undefined,
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
  const body = ghost ? (
    <span className="armada-run-node__ghost">{KIND[kind]}</span>
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
