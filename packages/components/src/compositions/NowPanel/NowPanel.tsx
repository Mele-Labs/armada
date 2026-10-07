import { useState, type ReactNode } from "react";
import { Bot, CircleDot, Megaphone, OctagonAlert, PanelRightClose, Scale, ShieldCheck, ShieldX } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Radio, RadioGroup } from "../../primitives/Radio/Radio";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * What a Job is doing and what it wants, beside its canvas: three sections, and
 * a section is drawn only when it holds a row. A Job with nothing in any of them
 * draws the head alone, and no sentence.
 *
 * **Every row is a press that opens the Drone, Check or Judge it names.** The
 * panel draws no surface of its own; the host says where each press goes.
 *
 * **Supplemental.** The lead keeps the one critical sentence. This panel is the
 * detail beside it, and hides from its own head.
 */
export type NowKind = "drone" | "check" | "judge";

/** A Plan decision the Drone needs made, with the answers it takes. Mock data until Fleet has a plan interview. */
export type NowPlanAsk = {
  key: string;
  kind: "plan";
  question: string;
  options: readonly { id: string; label: string }[];
  onAnswer: (optionId: string) => void;
};

/** A Judge's or a Drone's question, opened where it is answered. */
export type NowOpenAsk = {
  key: string;
  kind: "judge" | "drone";
  name: string;
  text: string;
  onOpen: () => void;
};

export type NowAsk = NowPlanAsk | NowOpenAsk;

export type NowIssue = {
  key: string;
  /** Which one it is about, and so which surface the press opens. */
  of: NowKind;
  name: string;
  text: string;
  /** The mark's tooltip: `Check failed`, `Drone stuck`. */
  said: string;
  onOpen: () => void;
};

export type NowRunning = {
  key: string;
  of: NowKind;
  name: string;
  /** A Drone's last action, live. Absent draws nothing. */
  line?: string;
  /** A Check lands as `passed` or `failed`; the rest are `running` until they leave the list. */
  state: "running" | "passed" | "failed";
  onOpen: () => void;
};

export type NowPanelProps = {
  asks?: readonly NowAsk[];
  issues?: readonly NowIssue[];
  running?: readonly NowRunning[];
  /** The head's hide button. Absent draws none. */
  onHide?: () => void;
};

const KIND: Record<NowKind, { Glyph: LucideIcon; said: string }> = {
  drone: { Glyph: Bot, said: "Drone" },
  check: { Glyph: ShieldCheck, said: "Check" },
  judge: { Glyph: Scale, said: "Judge" },
};

const STATE: Record<NowRunning["state"], { Glyph: LucideIcon; said: string }> = {
  running: { Glyph: CircleDot, said: "Running" },
  passed: { Glyph: ShieldCheck, said: "Passed" },
  failed: { Glyph: ShieldX, said: "Failed" },
};

export function NowPanel({ asks = [], issues = [], running = [], onHide }: NowPanelProps) {
  return (
    <aside className="armada-now" role="region" aria-label="Now">
      <header className="armada-now__head">
        <h3 className="armada-now__title">Now</h3>
        {onHide === undefined ? null : (
          <Tooltip label="Hide">
            <Button variant="ghost" size="sm" aria-label="Hide now" onClick={onHide}>
              <PanelRightClose size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </header>
      {asks.length === 0 ? null : (
        <Section tone="asks" label="Asks you">
          {asks.map((ask) => (ask.kind === "plan" ? <PlanAsk key={ask.key} ask={ask} /> : <AskRow key={ask.key} ask={ask} />))}
        </Section>
      )}
      {issues.length === 0 ? null : (
        <Section tone="issues" label="Issues">
          {issues.map((issue) => (
            <Row key={issue.key} name={`${issue.name}, ${issue.said}`} onOpen={issue.onOpen}>
              <Kind of={issue.of} />
              <span className="armada-now__text">{issue.text}</span>
              <Tooltip label={issue.said}>
                <span className="armada-now__mark" data-tone="issues" role="img" aria-label={issue.said}>
                  {issue.of === "check" ? <ShieldX size={12} strokeWidth={2} aria-hidden /> : <OctagonAlert size={12} strokeWidth={2} aria-hidden />}
                </span>
              </Tooltip>
            </Row>
          ))}
        </Section>
      )}
      {running.length === 0 ? null : (
        <Section tone="running" label="Running">
          {running.map((one) => {
            const state = STATE[one.state];
            return (
              <Row key={one.key} name={`${one.name}, ${state.said.toLowerCase()}`} onOpen={one.onOpen}>
                <Kind of={one.of} />
                <span className="armada-now__text">
                  {one.name}
                  {one.line === undefined ? null : <span className="armada-now__line">{one.line}</span>}
                </span>
                <Tooltip label={state.said}>
                  <span
                    className="armada-now__mark"
                    data-state={one.state}
                    data-pulsing={one.state === "running" || undefined}
                    role="img"
                    aria-label={state.said}
                  >
                    <state.Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </Row>
            );
          })}
        </Section>
      )}
    </aside>
  );
}

function Section({ tone, label, children }: { tone: "asks" | "issues" | "running"; label: string; children: ReactNode }) {
  return (
    <section className="armada-now__group" data-tone={tone} aria-label={label}>
      <h4 className="armada-now__band">{label}</h4>
      <ul className="armada-now__rows">{children}</ul>
    </section>
  );
}

function Kind({ of }: { of: NowKind }) {
  const { Glyph, said } = KIND[of];
  return (
    <Tooltip label={said}>
      <span className="armada-now__mark" role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function Row({ name, onOpen, children }: { name: string; onOpen: () => void; children: ReactNode }) {
  return (
    <li className="armada-now__row">
      <button type="button" className="armada-now__open" aria-label={`Open ${name}`} onClick={onOpen}>
        {children}
      </button>
    </li>
  );
}

function AskRow({ ask }: { ask: NowOpenAsk }) {
  return (
    <Row name={`${ask.name}, asks you`} onOpen={ask.onOpen}>
      <Kind of={ask.kind} />
      <span className="armada-now__text">{ask.text}</span>
      <Tooltip label="Asks you">
        <span className="armada-now__mark" data-tone="asks" role="img" aria-label="Asks you">
          <Megaphone size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </Row>
  );
}

/** Nothing is preselected: an answer commits the Drone to a branch of the plan. */
function PlanAsk({ ask }: { ask: NowPlanAsk }) {
  const [picked, setPicked] = useState<string | undefined>();
  const [sent, setSent] = useState(false);
  return (
    <li className="armada-now__ask">
      <RadioGroup label={ask.question}>
        {ask.options.map((option) => (
          <Radio
            key={option.id}
            name={ask.key}
            value={option.id}
            checked={picked === option.id}
            disabled={sent}
            onChange={() => setPicked(option.id)}
          >
            {option.label}
          </Radio>
        ))}
      </RadioGroup>
      <Button
        size="sm"
        disabled={picked === undefined || sent}
        onClick={() => {
          if (picked === undefined) return;
          setSent(true);
          ask.onAnswer(picked);
        }}
      >
        Answer
      </Button>
    </li>
  );
}
