import type { CSSProperties } from "react";
import { GitBranch, GitPullRequest, ScrollText } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

// One node of a Job's run on the approval canvas, each kind drawn as its own
// object (the owner, 4 Oct 2026: the nodes "carry no weight"). Prototype: no
// story until he has walked it. Status hue comes from the mark alone.

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

/** One trait on a node's face: what it is called, small, and its value. */
export type RunNodeTrait = { key: string; value: string; mono?: boolean };

/** `ahead` is the gate, where nothing is dimmed; `upcoming` is a running Job's node not reached yet. */
export type RunNodeState = "ahead" | "upcoming" | "live" | "done";

export type RunNodeProps = {
  kind: RunNodeKind;
  name: string;
  /** The big words on the node's face where they are not its name — the brief's title, the base's branch. */
  face?: string;
  /** Its place among the workflow's steps, from one. Steps alone carry one. */
  ordinal?: number;
  /** One line under the name: a title, a branch, a mode. */
  line?: string;
  /** A value a person copies — a branch — so mono. */
  lineMono?: boolean;
  traits?: readonly RunNodeTrait[];
  /** Done when's criteria; the first three are drawn. */
  items?: readonly string[];
  activity: StepActivity;
  /** The registry's word for the state, read to somebody who cannot see the mark. */
  said: string;
  /** A registry row's own glyph and token, in place of the step mark. */
  mark?: { icon: LucideIcon; token: string };
  state: RunNodeState;
  selected?: boolean;
  /** Absent draws a node that is not a control. */
  onOpen?: () => void;
};

/** Each kind's height as `RunNode.css` declares it off the tokens; a number because React Flow places by number. */
export const RUN_NODE_HEIGHT: Readonly<Record<RunNodeKind, number>> = {
  brief: 90,
  base: 38,
  start: 48,
  step: 96,
  gate: 38,
  stack: 38,
  fan: 38,
  group: 38,
  job: 58,
  done: 114,
  pr: 38,
  land: 117,
};

/** Each kind's width, as its token: `--w-workflow-node` and the two steps below it. */
export const RUN_NODE_WIDTH: Readonly<Record<RunNodeKind, number>> = {
  brief: 260,
  base: 196,
  start: 196,
  step: 260,
  gate: 260,
  stack: 228,
  fan: 228,
  group: 228,
  job: 228,
  done: 260,
  pr: 260,
  land: 260,
};

/** The run's substance, lifted off the canvas as glass. */
const GLASS: ReadonlySet<RunNodeKind> = new Set(["brief", "step", "done", "land"]);

/** The kinds with a registry glyph of their own; every other kind is its silhouette. */
const GLYPH: Partial<Record<RunNodeKind, LucideIcon>> = {
  base: GitBranch,
  start: ScrollText,
  pr: GitPullRequest,
};

export function RunNode({
  kind,
  name,
  face,
  ordinal,
  line,
  lineMono = false,
  traits = [],
  items = [],
  activity,
  said,
  mark,
  state,
  selected = false,
  onOpen,
}: RunNodeProps) {
  const named = `${name}, ${said}`;
  const Glyph = GLYPH[kind];
  // A ref names its branch in mono.
  const ref = kind === "base" && face !== undefined;
  // What a running Job has not reached stays on the canvas until the work gets there.
  const glass = GLASS.has(kind) && state !== "upcoming";
  const attributes = {
    className: glass ? "armada-run-node armada-glass" : "armada-run-node",
    "data-kind": kind,
    "data-state": state,
    "data-activity": activity,
  };
  const body = (
    <>
      <span className="armada-run-node__head">
        {mark === undefined ? (
          <StepActivityMark activity={activity} label={said} ordinal={ordinal} says={named} />
        ) : (
          <Tooltip asChild label={named}>
            <span
              className="armada-step-mark armada-run-node__mark"
              style={{ "--armada-run-node-mark": `var(${mark.token})` } as CSSProperties}
            >
              <mark.icon size={12} strokeWidth={2} aria-hidden />
              <span className="armada-step-mark__name">{said}</span>
            </span>
          </Tooltip>
        )}
        {Glyph === undefined ? null : <Glyph className="armada-run-node__glyph" size={16} strokeWidth={2} aria-hidden />}
        <span className="armada-run-node__name" data-ref={ref || undefined}>
          {face ?? name}
        </span>
      </span>
      {line === undefined ? null : (
        <span className="armada-run-node__line" data-mono={lineMono || undefined}>
          {line}
        </span>
      )}
      {items.length === 0 ? null : (
        <span className="armada-run-node__items">
          {items.slice(0, 3).map((item) => (
            <span key={item} className="armada-run-node__item">
              {item}
            </span>
          ))}
        </span>
      )}
      {traits.length === 0 ? null : (
        <span className="armada-run-node__traits">
          {traits.map((trait) => (
            <span key={trait.key} className="armada-run-node__trait">
              <span className="armada-run-node__trait-key">{trait.key}</span>
              <span className="armada-run-node__trait-value" data-mono={trait.mono || undefined}>
                {trait.value}
              </span>
            </span>
          ))}
        </span>
      )}
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
