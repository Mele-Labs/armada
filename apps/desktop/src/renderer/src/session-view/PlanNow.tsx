// Now for a plan that ends on gates: one row a chunk still to finish, a line a group and the gate's glyph on the last, and under the live chunk what ends it and what is being
// done now. A finished chunk stands in Done, headed by `GateClosed`. Every chunk's tasks are folded until pressed. ↑ ↓ move between the rows (`arrowsIn`).

import { useState, type ReactNode } from "react";
import {
  Check,
  ChevronDown,
  ChevronRight,
  CircleDashed,
  CircleCheck,
  CircleDot,
  Drone,
  Footprints,
  GitMerge,
  Link,
  NotebookPen,
  PencilRuler,
  ShieldCheck,
  ShieldMinus,
  ShieldX,
  Stamp as StampGlyph,
  type LucideIcon,
} from "lucide-react";
import { Stamp, Tooltip } from "@armada/components";
import type { WorkGroup } from "@armada/screens/src/draft/sessions";
import {
  GATE_ENDS,
  type GateKind,
  type Stretch,
  type StretchGate,
  type StretchState,
} from "./stretches";
import { Since } from "./StoryParts";
import "./plan-now.css";

export const GATE_GLYPH: Record<GateKind, LucideIcon> = {
  plan_approved: StampGlyph,
  walk_approved: Footprints,
  sketch_approved: PencilRuler,
  document_approved: NotebookPen,
  you_approved: CircleCheck,
  checks_green: ShieldCheck,
  merged: GitMerge,
  // The Jobs tab's glyph: the Jobs this one started.
  jobs_ended: Link,
};

const STATE_GLYPH: Record<StretchState, LucideIcon> = {
  done: Check,
  now: CircleDot,
  next: CircleDashed,
};
const STATE_SAID: Record<StretchState, string> = {
  done: "Done",
  now: "Now",
  next: "Next",
};

type GateRead = {
  said: string;
  mark: "done" | "failed" | "waiting" | "next";
  /** The gate glyph's tooltip. */
  glyph: string;
  failed: readonly string[];
  red: readonly string[];
};

/** What a gate says: the fact Fleet saw close it; while open, its Checks at the pushed commit, or what it waits for. */
const readGate = (gate: StretchGate, state: StretchState): GateRead => {
  const closed = gate.closed;
  const checks = closed === undefined ? gate.checks : undefined;
  return {
    said:
      closed !== undefined
        ? closed.fact
        : checks === undefined
          ? `Ends when ${GATE_ENDS[gate.kind]}`
          : checks.state === "running"
            ? `Checks running at ${checks.commit}`
            : `Checks failed at ${checks.commit}`,
    mark:
      closed !== undefined
        ? "done"
        : checks?.state === "red"
          ? "failed"
          : state === "now"
            ? "waiting"
            : "next",
    glyph:
      closed !== undefined
        ? "Gate, closed by Fleet"
        : gate.held !== undefined
          ? "Gate, the Job is held on you"
          : `Ends when ${GATE_ENDS[gate.kind]}`,
    failed: checks?.state === "red" ? (checks.failed ?? []) : [],
    red: closed?.already_red ?? checks?.already_red ?? [],
  };
};

/** The tests a Checks gate names: those that hold it, then those red on main too. */
function GateTests({ read }: { read: GateRead }) {
  return read.failed.length === 0 && read.red.length === 0 ? null : (
    <ul className="sv-gate__red" aria-label="Tests">
      {read.failed.map((test) => (
        <li key={`failed-${test}`}>
          <Tooltip asChild label="Failed here, and passes on main: holds the gate">
            <span className="sv-gate__test" data-failed="" tabIndex={0}>
              <ShieldX size={12} strokeWidth={2} aria-hidden />
              <span>{test}</span>
            </span>
          </Tooltip>
        </li>
      ))}
      {read.red.map((test) => (
        <li key={`red-${test}`}>
          <Tooltip asChild label="Red on main too, so it does not hold the gate">
            <span className="sv-gate__test" tabIndex={0}>
              <ShieldMinus size={12} strokeWidth={2} aria-hidden />
              <span>{test}</span>
            </span>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

/** At the head of a finished chunk in Done: the gate that ended it, the fact Fleet saw close it, and when. */
export function GateClosed({ gate }: { gate: StretchGate | undefined }) {
  if (gate?.closed === undefined) return null;
  const Gate = GATE_GLYPH[gate.kind];
  const read = readGate(gate, "done");
  return (
    <div className="sv-gate-closed">
      <Tooltip asChild label={read.glyph}>
        <span className="sv-gate-closed__glyph" role="img" aria-label={read.glyph}>
          <Gate size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-chunk__fact">{read.said}</span>
      <Stamp at={gate.closed.at} className="sv-beat__figure" />
      <GateTests read={read} />
    </div>
  );
}

/** A subagent at work, with the plan group its brief named where it named one. */
export type HelperRow = {
  id: string;
  task: string;
  /** The group's title. */
  group?: string;
};

/** The subagents at work in the live chunk: each one's task, and the group it works on. A press opens its thread. */
export function Helpers({
  rows,
  onOpen,
}: {
  rows: readonly HelperRow[];
  onOpen: (id: string) => void;
}) {
  return rows.length === 0 ? null : (
    <ul className="sv-subagents" aria-label="Subagents at work">
      {rows.map((one) => (
        <li key={one.id}>
          <button
            type="button"
            className="sv-subagents__row sv-chunk__helper"
            data-nav=""
            onClick={() => onOpen(one.id)}
          >
            <Tooltip asChild label="Subagent">
              <span className="sv-chunk__drone" role="img" aria-label="Subagent">
                <Drone size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <span className="sv-subagents__task">
              {one.task}
              {one.group === undefined ? null : <span className="sv-chunk__on">{one.group}</span>}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

/** One row a chunk. The live one holds what is being done now and a Tasks press; any other opens its tasks on a press. */
export function PlanChunks<G extends WorkGroup>({
  stretches,
  doing,
  tasksOf,
}: {
  stretches: readonly Stretch<G>[];
  /** What stands under the live chunk. */
  doing: ReactNode;
  /** A chunk's tasks, drawn when it is opened. */
  tasksOf: (stretch: Stretch<G>) => ReactNode;
}) {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const toggle = (key: string) =>
    setOpen((was) => {
      const next = new Set(was);
      if (!next.delete(key)) next.add(key);
      return next;
    });
  return (
    <ol className="sv-chunks" aria-label="Plan">
      {stretches.map((stretch) => {
        const State = STATE_GLYPH[stretch.state];
        const gate = stretch.gate;
        const Gate = gate === undefined ? undefined : GATE_GLYPH[gate.kind];
        const read = gate === undefined ? undefined : readGate(gate, stretch.state);
        const live = stretch.state === "now";
        // Opened is kept by chunk and state, so a chunk that moves on folds again.
        const at = `${stretch.key}:${stretch.state}`;
        const opened = open.has(at);
        const head = (
          <>
            <Tooltip asChild label={STATE_SAID[stretch.state]}>
              <span className="sv-chunk__state" role="img" aria-label={STATE_SAID[stretch.state]}>
                <State size={14} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            {/* One line a group, and the gate's glyph on the last: one gate ends them all. */}
            <span className="sv-chunk__name">
              {stretch.groups.map((group) => (
                <span key={group.id} className="sv-chunk__group">
                  {group.title}
                </span>
              ))}
            </span>
            {gate === undefined || Gate === undefined || read === undefined ? null : (
              <Tooltip asChild label={read.glyph}>
                <span
                  className="sv-chunk__gate"
                  role="img"
                  aria-label={read.glyph}
                  style={{
                    gridRow: stretch.groups.length,
                  }}
                >
                  <Gate size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            {/* Only the live chunk, or one opened, says its gate here; the rest say it in the glyph's tooltip. The time wraps under the fact rather than squeeze it. */}
            {read === undefined || gate === undefined || (!live && !opened) ? null : (
              <span className="sv-chunk__said">
                <span className="sv-chunk__fact">{read.said}</span>
                {gate.closed !== undefined ? (
                  <Stamp at={gate.closed.at} className="sv-beat__figure" />
                ) : gate.held !== undefined ? (
                  <Since at={gate.held} said="Held on you for" />
                ) : null}
              </span>
            )}
          </>
        );
        return (
          <li
            key={stretch.key}
            className="sv-chunk"
            data-state={stretch.state}
            data-gate={read?.mark}
            data-lamp={gate?.held === undefined ? undefined : "needs"}
            aria-current={live ? "step" : undefined}
          >
            {live ? (
              <div className="sv-chunk__row">{head}</div>
            ) : (
              <button
                type="button"
                className="sv-chunk__row"
                data-nav=""
                aria-expanded={opened}
                onClick={() => toggle(at)}
              >
                {head}
              </button>
            )}
            {read === undefined ? null : <GateTests read={read} />}
            {live ? (
              <div className="sv-chunk__under">
                <section className="sv-chunk__doing" aria-label="Doing now">
                  {doing}
                </section>
                <button
                  type="button"
                  className="sv-chunk__tasks"
                  data-nav=""
                  aria-expanded={opened}
                  onClick={() => toggle(at)}
                >
                  {opened ? (
                    <ChevronDown size={14} strokeWidth={2} aria-hidden />
                  ) : (
                    <ChevronRight size={14} strokeWidth={2} aria-hidden />
                  )}
                  Tasks
                </button>
                {opened ? tasksOf(stretch) : null}
              </div>
            ) : opened ? (
              <div className="sv-chunk__under">{tasksOf(stretch)}</div>
            ) : null}
          </li>
        );
      })}
    </ol>
  );
}
