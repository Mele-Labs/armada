// The plan as the Job's progress line, at the head of its story: each stretch across, ended by its gate's glyph, the live stretch lit. A press on a stretch, or Enter on it, puts its
// zone at the top of the spine; ← → Home End move along the line. Under each zone the gate stands as its last row (`GateBeat`): what it waits for, or what Fleet saw close it.

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
import "./gate-line.css";

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

/** What a gate's glyph says on hover: what it waits for, or the fact that closed it. */
const gateSaid = (gate: StretchGate): string =>
  gate.closed?.fact ??
  (gate.held === undefined ? `Waits for: ${GATE_SAID[gate.kind]}` : `Held on you: ${GATE_SAID[gate.kind]}`);

export function GateLine({
  stretches,
  onPick,
}: {
  stretches: readonly Stretch[];
  onPick: (key: string) => void;
}) {
  const list = useRef<HTMLOListElement>(null);
  const live = stretches.find((one) => one.state === "now")?.key ?? stretches[0]?.key;
  const move = (event: KeyboardEvent<HTMLButtonElement>) => {
    const buttons = [
      ...(list.current?.querySelectorAll<HTMLButtonElement>(".sv-gates__stretch") ?? []),
    ];
    const at = buttons.indexOf(event.currentTarget);
    const to =
      event.key === "ArrowRight"
        ? Math.min(at + 1, buttons.length - 1)
        : event.key === "ArrowLeft"
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
    <ol ref={list} className="sv-gates" aria-label="Progress">
      {stretches.map((stretch) => {
        const Glyph = STATE_GLYPH[stretch.state];
        const Gate = stretch.gate === undefined ? undefined : GATE_GLYPH[stretch.gate.kind];
        return (
          <li key={stretch.key} className="sv-gates__step" data-state={stretch.state}>
            <Tooltip asChild label={`${stretch.name} · ${STATE_SAID[stretch.state]}`}>
              <button
                type="button"
                className="sv-gates__stretch"
                aria-label={stretch.name}
                aria-current={stretch.state === "now" ? "step" : undefined}
                tabIndex={stretch.key === live ? 0 : -1}
                onKeyDown={move}
                onClick={() => onPick(stretch.key)}
              >
                <span className="sv-gates__bar" aria-hidden />
                <span className="sv-gates__name">
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                  {stretch.state === "now" ? (
                    <span className="sv-gates__text">{stretch.name}</span>
                  ) : null}
                </span>
              </button>
            </Tooltip>
            {stretch.gate === undefined || Gate === undefined ? null : (
              <Tooltip asChild label={gateSaid(stretch.gate)}>
                <span
                  className="sv-gates__gate"
                  role="img"
                  aria-label={gateSaid(stretch.gate)}
                  data-closed={stretch.gate.closed === undefined ? undefined : ""}
                  data-failed={
                    stretch.gate.closed === undefined && stretch.gate.checks?.state === "red"
                      ? ""
                      : undefined
                  }
                  data-lamp={stretch.gate.held === undefined ? undefined : "needs"}
                >
                  <Gate size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
          </li>
        );
      })}
    </ol>
  );
}

/** A stretch's gate as the last row of its zone: the gate's glyph, then what it waits for, or the fact Fleet saw close it and when. */
export function GateBeat({ gate, state }: { gate: StretchGate; state: StretchState }) {
  const Gate = GATE_GLYPH[gate.kind];
  const closed = gate.closed;
  // While a Checks gate is open, Fleet's run at the pushed commit stands on its row: running, or red with what failed.
  const checks = closed === undefined ? gate.checks : undefined;
  const red = closed?.already_red ?? checks?.already_red ?? [];
  const failed = checks?.state === "red" ? (checks.failed ?? []) : [];
  const said =
    closed !== undefined
      ? closed.fact
      : checks === undefined
        ? GATE_SAID[gate.kind]
        : checks.state === "running"
          ? `Checks running at ${checks.commit}`
          : `Checks failed at ${checks.commit}`;
  return (
    <li
      className="sv-beat sv-gate"
      {...(gate.held === undefined
        ? {}
        : {
            "data-lamp": "needs",
          })}
      data-state={
        closed !== undefined
          ? "done"
          : checks?.state === "red"
            ? "failed"
            : state === "now"
              ? "waiting"
              : "next"
      }
    >
      <Tooltip
        asChild
        label={
          closed !== undefined
            ? "Gate, closed by Fleet"
            : gate.held !== undefined
              ? "Gate, the Job is held on you"
              : "Gate, open"
        }
      >
        <span
          className="armada-session-mark sv-beat__lead"
          role="img"
          aria-label="Gate"
          {...(gate.held === undefined
            ? {}
            : {
                "data-pulsing": "",
              })}
        >
          <Gate size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <span className="sv-beat__label">{said}</span>
      {closed !== undefined ? (
        <Stamp at={closed.at} className="sv-beat__figure" />
      ) : gate.held !== undefined ? (
        <Since at={gate.held} said="Held on you for" />
      ) : (
        <span />
      )}
      {failed.length === 0 && red.length === 0 ? null : (
        <ul className="sv-gate__red" aria-label="Tests">
          {failed.map((test) => (
            <li key={`failed-${test}`}>
              <Tooltip asChild label="Failed here, and passes on main: holds the gate">
                <span className="sv-gate__test" data-failed="" tabIndex={0}>
                  <ShieldX size={12} strokeWidth={2} aria-hidden />
                  <span>{test}</span>
                </span>
              </Tooltip>
            </li>
          ))}
          {red.map((test) => (
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
      )}
    </li>
  );
}
