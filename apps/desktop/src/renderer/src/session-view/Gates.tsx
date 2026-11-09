// The plan as the Job's progress, drawn two ways while he picks one. `GateStop`: each gate stands on the spine between its stretch's zone and the next, so a folded zone still shows
// what closed it. `GateList`: the stretches as a list at the head of the story, each with its gate, and a press or Enter puts that stretch's zone at the top of the spine.

import { useRef, type KeyboardEvent } from "react";
import {
  CircleCheck,
  CircleDashed,
  CircleDot,
  Footprints,
  GitMerge,
  NotebookPen,
  PencilRuler,
  ShieldCheck,
  ShieldMinus,
  ShieldX,
  Stamp as StampGlyph,
  type LucideIcon,
} from "lucide-react";
import { Stamp, Tooltip } from "@armada/components";
import {
  GATE_SAID,
  type GateKind,
  type Stretch,
  type StretchGate,
  type StretchState,
} from "./stretches";
import { Since } from "./StoryParts";
import "./gates.css";

export const GATE_GLYPH: Record<GateKind, LucideIcon> = {
  plan_approved: StampGlyph,
  walk_approved: Footprints,
  sketch_approved: PencilRuler,
  document_approved: NotebookPen,
  checks_green: ShieldCheck,
  merged: GitMerge,
};

const STATE_GLYPH: Record<StretchState, LucideIcon> = {
  done: CircleCheck,
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
          ? GATE_SAID[gate.kind]
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
          : "Gate, open",
    failed: checks?.state === "red" ? (checks.failed ?? []) : [],
    red: closed?.already_red ?? checks?.already_red ?? [],
  };
};

/** When it closed, or how long the Job has been held on him at it. */
function GateWhen({ gate }: { gate: StretchGate }) {
  return gate.closed !== undefined ? (
    <Stamp at={gate.closed.at} className="sv-beat__figure" />
  ) : gate.held !== undefined ? (
    <Since at={gate.held} said="Held on you for" />
  ) : null;
}

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

/**
 * A stretch's gate on the spine, under its zone: the gate's glyph on the line, then what it waits for, or the fact Fleet saw close it and when. `area` is its zone's, so a pane
 * showing that zone shows its gate too.
 */
export function GateStop({
  gate,
  state,
}: {
  gate: StretchGate;
  state: StretchState;
  area: string;
}) {
  const Gate = GATE_GLYPH[gate.kind];
  const read = readGate(gate, state);
  return (
    <div
      className="sv-stop"
      role="group"
      aria-label={`Gate: ${read.said}`}
      data-state={read.mark}
      data-lamp={gate.held === undefined ? undefined : "needs"}
    >
      <Tooltip asChild label={read.glyph}>
        <span className="sv-stop__mark" role="img" aria-label={read.glyph}>
          <Gate size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-stop__said">{read.said}</span>
      <GateWhen gate={gate} />
      <GateTests read={read} />
    </div>
  );
}

/** The stretches as a list at the story's head: each one's state, its name whole, what its gate waits for or the fact that closed it, and the gate's glyph. ↑ ↓ Home End move. */
export function GateList({
  stretches,
  onPick,
}: {
  stretches: readonly Stretch[];
  onPick: (key: string) => void;
}) {
  const list = useRef<HTMLOListElement>(null);
  const live = stretches.find((one) => one.state === "now")?.key ?? stretches[0]?.key;
  const move = (event: KeyboardEvent<HTMLButtonElement>) => {
    const buttons = [...(list.current?.querySelectorAll<HTMLButtonElement>(".sv-miles__row") ?? [])];
    const at = buttons.indexOf(event.currentTarget);
    const to =
      event.key === "ArrowDown"
        ? Math.min(at + 1, buttons.length - 1)
        : event.key === "ArrowUp"
          ? Math.max(at - 1, 0)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? buttons.length - 1
              : undefined;
    if (to === undefined) return;
    event.preventDefault();
    buttons[to]?.focus();
  };
  return (
    <ol ref={list} className="sv-miles" aria-label="Progress">
      {stretches.map((stretch) => {
        const State = STATE_GLYPH[stretch.state];
        const gate = stretch.gate;
        const Gate = gate === undefined ? undefined : GATE_GLYPH[gate.kind];
        const read = gate === undefined ? undefined : readGate(gate, stretch.state);
        return (
          <li
            key={stretch.key}
            className="sv-miles__item"
            data-state={stretch.state}
            data-gate={read?.mark}
            data-lamp={gate?.held === undefined ? undefined : "needs"}
          >
            <button
              type="button"
              className="sv-miles__row"
              aria-current={stretch.state === "now" ? "step" : undefined}
              tabIndex={stretch.key === live ? 0 : -1}
              onKeyDown={move}
              onClick={() => onPick(stretch.key)}
            >
              <Tooltip asChild label={STATE_SAID[stretch.state]}>
                <span className="sv-miles__state" role="img" aria-label={STATE_SAID[stretch.state]}>
                  <State size={14} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              <span className="sv-miles__name">{stretch.name}</span>
              {read === undefined ? null : <span className="sv-miles__said">{read.said}</span>}
            </button>
            {gate === undefined || Gate === undefined || read === undefined ? null : (
              <span className="sv-miles__end">
                <GateWhen gate={gate} />
                <Tooltip asChild label={read.glyph}>
                  <span className="sv-miles__gate" role="img" aria-label={read.glyph}>
                    <Gate size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </span>
            )}
            {read === undefined ? null : <GateTests read={read} />}
          </li>
        );
      })}
    </ol>
  );
}
