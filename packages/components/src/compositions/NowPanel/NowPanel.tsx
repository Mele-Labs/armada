import { Fragment, useEffect, useState, type ReactNode } from "react";
import { Bot, Box, ChevronDown, ChevronRight, CircleDot, CornerUpRight, Cpu, Megaphone, OctagonAlert, PanelRightClose, PencilRuler, RotateCw, Scale, ShieldCheck, ShieldOff, ShieldX, SkipForward, Waypoints, Workflow } from "lucide-react";
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

/**
 * A diagram the asking Drone drew to go with its question. `source` is the mermaid text the Drone
 * wrote; `svg` is that source drawn. **Mock only until a renderer is chosen**: the mock carries a
 * pre-drawn `svg` and nothing here parses `source`.
 */
export type NowSketch = { source: string; svg: string };

/** What the Overview shows to the left of the panel while a sketch is open. */
export type NowSketchView = "sketch" | "canvas";

/** A Plan decision the Drone needs made, with the answers it takes. Mock data until Fleet has a plan interview. */
export type NowDecision = {
  id: string;
  question: string;
  options: readonly { id: string; label: string }[];
  /** Shown beside the canvas while this decision is the one asked. */
  sketch?: NowSketch;
};

/** Decisions are asked one at a time. The answer goes once, at the last, as an option id per decision. */
export type NowPlanAsk = {
  key: string;
  kind: "plan";
  decisions: readonly NowDecision[];
  onAnswer: (answers: Readonly<Record<string, string>>) => void;
};

/** A Judge's or a Drone's question, opened where it is answered. */
export type NowOpenAsk = {
  key: string;
  kind: "judge" | "drone";
  name: string;
  text: string;
  /** Shown beside the canvas while the question is open. */
  sketch?: NowSketch;
  onOpen: () => void;
};

export type NowAsk = NowPlanAsk | NowOpenAsk;

/**
 * A quick act on a row, as an icon with a tooltip: retry a Check, skip it, redirect a stuck Drone,
 * retry a step. The host decides what each does.
 */
export type NowAct = {
  key: string;
  glyph: "retry" | "skip" | "skip_all" | "redirect" | "retry_step";
  /** The tooltip, and the button's name. */
  said: string;
  onAct: () => void;
};

const ACT: Record<NowAct["glyph"], LucideIcon> = {
  retry: RotateCw,
  skip: SkipForward,
  skip_all: ShieldOff,
  redirect: CornerUpRight,
  retry_step: RotateCw,
};

/**
 * Why nothing is running, as a row: a resource it waits on, another Job it waits on, steps in
 * transition, or a one-off step it runs. A Job is never idle without one of these.
 */
export type NowWaiting = {
  key: string;
  kind: "resource" | "job" | "transition" | "step";
  /** The resource, the Job, the steps or the step, named. */
  text: string;
  /** The canvas step it is about, which stays lit there. */
  step?: { id: string; name: string };
  /** Present on a row that opens something: a Job, a step. */
  onOpen?: () => void;
};

const WAITING: Record<NowWaiting["kind"], { Glyph: LucideIcon; said: string }> = {
  resource: { Glyph: Cpu, said: "Resource" },
  job: { Glyph: Box, said: "Job" },
  transition: { Glyph: Waypoints, said: "Transition" },
  step: { Glyph: Workflow, said: "Step" },
};

export type NowIssue = {
  key: string;
  /** Which one it is about, and so which surface the press opens. */
  of: NowKind;
  name: string;
  text: string;
  /** The mark's tooltip: `Check failed`, `Drone stuck`. */
  said: string;
  /** The canvas step it is about, which stays lit there. */
  step?: { id: string; name: string };
  /** What fixes it, one icon each. */
  acts?: readonly NowAct[];
  onOpen: () => void;
};

export type NowRunning = {
  key: string;
  of: NowKind;
  name: string;
  /** A Drone's last action, live. Absent draws nothing. */
  line?: string;
  /** The canvas step this belongs to. Pressing it asks the host to highlight that step. */
  step?: { id: string; name: string };
  /** The last few lines of its output, oldest first. Folded until pressed. Absent draws no toggle. */
  tail?: readonly string[];
  /** A Check lands as `passed` or `failed`; the rest are `running` until they leave the list. */
  state: "running" | "passed" | "failed";
  /** Quick acts: a Check's retry and skip. */
  acts?: readonly NowAct[];
  onOpen: () => void;
};

export type NowPanelProps = {
  asks?: readonly NowAsk[];
  issues?: readonly NowIssue[];
  running?: readonly NowRunning[];
  /** Why nothing is running, where nothing is. */
  waiting?: readonly NowWaiting[];
  /** Skip every Check of the run. Drawn in the Running band, over Check rows only. */
  onSkipAll?: () => void;
  /** The head's hide button. Absent draws none. */
  onHide?: () => void;
  /** A running row's step was pressed. Absent draws the step as plain text. */
  onStep?: (stepId: string) => void;
  /** The step highlighted on the canvas now. */
  focusedStep?: string;
  /** Told the sketch of the ask now open, and `undefined` once none is. Absent leaves sketches undrawn. */
  onSketch?: (sketch: NowSketch | undefined) => void;
  /** Which the Overview shows while a sketch is open. Absent reads as the sketch. */
  sketchView?: NowSketchView;
  /** The head's Sketch and Canvas switch, drawn only while a sketch is open. Absent draws none. */
  onSketchView?: (view: NowSketchView) => void;
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

const ASK_ORDER = ["plan", "judge", "drone"] as const;
const RUN_ORDER = ["drone", "check", "judge"] as const;

export function NowPanel({ asks = [], issues = [], running = [], waiting = [], onSkipAll, onHide, onStep, focusedStep, onSketch, sketchView = "sketch", onSketchView }: NowPanelProps) {
  // The plan decision asked now reports its own sketch; a Judge's or Drone's is read off the asks.
  const [planSketch, setPlanSketch] = useState<NowSketch | undefined>(undefined);
  const asked = planSketch ?? asks.flatMap((ask) => (ask.kind === "plan" || ask.sketch === undefined ? [] : [ask.sketch]))[0];
  useEffect(() => {
    onSketch?.(asked);
    return () => onSketch?.(undefined);
  }, [asked, onSketch]);
  const drones = running.filter((one) => one.of === "drone");
  const checks = running.some((one) => one.of === "check");
  // **One Drone at work shows its live view**, its output tail open (owner, 6 Oct 2026).
  const alone = drones.length === 1 ? drones[0]?.key : undefined;
  const nothing = asks.length + issues.length + running.length + waiting.length === 0;
  return (
    <aside className="armada-now" role="region" aria-label="Now">
      <header className="armada-now__head">
        <h3 className="armada-now__title">Now</h3>
        {asked === undefined || onSketchView === undefined ? null : (
          <div className="armada-now__switch" role="group" aria-label="Show on the left">
            <Tooltip label="Sketch">
              <button type="button" className="armada-now__switch-act" aria-label="Sketch" aria-pressed={sketchView === "sketch"} onClick={() => onSketchView("sketch")}>
                <PencilRuler size={16} strokeWidth={2} aria-hidden />
              </button>
            </Tooltip>
            <Tooltip label="Canvas">
              <button type="button" className="armada-now__switch-act" aria-label="Canvas" aria-pressed={sketchView === "canvas"} onClick={() => onSketchView("canvas")}>
                <Workflow size={16} strokeWidth={2} aria-hidden />
              </button>
            </Tooltip>
          </div>
        )}
        {onHide === undefined ? null : (
          <Tooltip label="Hide">
            <Button variant="ghost" size="sm" aria-label="Hide now" onClick={onHide}>
              <PanelRightClose size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        )}
      </header>
      {/* **A Job is never idle without a reason.** This sentence is a defect made visible: the
          owner reads it as a bug in whatever left the Job with nothing to say (8 Oct 2026). */}
      {!nothing ? null : <p className="armada-now__nothing">Nothing is actively running on this job</p>}
      {asks.length === 0 ? null : (
        <Section tone="asks" label="Asks you">
          {ASK_ORDER.map((kind) => {
            const of = asks.filter((ask) => ask.kind === kind);
            return of.length === 0 ? null : (
              <Fragment key={kind}>
                {of.map((ask) => (ask.kind === "plan" ? <PlanAsk key={ask.key} ask={ask} onSketch={setPlanSketch} /> : <AskRow key={ask.key} ask={ask} />))}
              </Fragment>
            );
          })}
        </Section>
      )}
      {issues.length === 0 ? null : (
        <Section tone="issues" label="Issues">
          {issues.map((issue) => (
            <Row
              key={issue.key}
              name={`${issue.name}, ${issue.said}`}
              onOpen={issue.onOpen}
              below={<Acts step={issue.step} acts={issue.acts} />}
            >
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
        <Section
          tone="running"
          label="Running"
          {...(onSkipAll === undefined || !checks
            ? {}
            : {
                trailing: (
                  <Tooltip label="Skip all checks">
                    <button type="button" className="armada-now__band-act" aria-label="Skip all checks" onClick={onSkipAll}>
                      <ShieldOff size={12} strokeWidth={2} aria-hidden />
                    </button>
                  </Tooltip>
                ),
              })}
        >
          {RUN_ORDER.map((kind) => {
            const of = running.filter((one) => one.of === kind);
            return of.length === 0 ? null : (
              <Fragment key={kind}>
                {of.map((one) => (
                  <RunningRow
                    key={one.key}
                    one={one}
                    {...(onStep === undefined ? {} : { onStep })}
                    focused={one.step !== undefined && one.step.id === focusedStep}
                    open={one.key === alone}
                  />
                ))}
              </Fragment>
            );
          })}
        </Section>
      )}
      {waiting.length === 0 ? null : (
        <Section tone="waiting" label="Waiting">
          {waiting.map((one) => {
            const kind = WAITING[one.kind];
            return (
              <Row
                key={one.key}
                name={`${kind.said} ${one.text}`}
                {...(one.onOpen === undefined ? {} : { onOpen: one.onOpen })}
              >
                <Tooltip label={kind.said}>
                  <span className="armada-now__mark" role="img" aria-label={kind.said}>
                    <kind.Glyph size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                <span className="armada-now__text">{one.text}</span>
              </Row>
            );
          })}
        </Section>
      )}
    </aside>
  );
}

/** The icons that fix a row. A step in the row is a label here, unless the host asked to hear presses. */
function Acts({ step, acts }: { step?: { id: string; name: string } | undefined; acts?: readonly NowAct[] | undefined }) {
  if ((acts === undefined || acts.length === 0) && step === undefined) return null;
  return (
    <div className="armada-now__sub">
      {step === undefined ? null : <span className="armada-now__step">{step.name}</span>}
      {(acts ?? []).map((act) => {
        const Glyph = ACT[act.glyph];
        return (
          <Tooltip key={act.key} label={act.said}>
            <button type="button" className="armada-now__act" aria-label={act.said} onClick={act.onAct}>
              <Glyph size={12} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
        );
      })}
    </div>
  );
}

function Section({ tone, label, trailing, children }: { tone: "asks" | "issues" | "running" | "waiting"; label: string; trailing?: ReactNode; children: ReactNode }) {
  return (
    <section className="armada-now__group" data-tone={tone} aria-label={label}>
      <div className="armada-now__band">
        <h4 className="armada-now__band-title">{label}</h4>
        {trailing}
      </div>
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

function Row({ name, onOpen, children, below }: { name: string; onOpen?: () => void; children: ReactNode; below?: ReactNode }) {
  return (
    <li className="armada-now__row">
      {onOpen === undefined ? (
        <div className="armada-now__open" data-static role="group" aria-label={name}>
          {children}
        </div>
      ) : (
        <button type="button" className="armada-now__open" aria-label={`Open ${name}`} onClick={onOpen}>
          {children}
        </button>
      )}
      {below}
    </li>
  );
}

/** A Drone, Check or Judge at work: its live mark, the step it belongs to, and its output tail folded under it. */
function RunningRow({ one, onStep, focused, open }: { one: NowRunning; onStep?: (stepId: string) => void; focused: boolean; open: boolean }) {
  const [shown, setShown] = useState(open);
  const state = STATE[one.state];
  const tail = one.tail === undefined || one.tail.length === 0 ? undefined : one.tail;
  const step = one.step;
  return (
    <Row
      name={`${one.name}${one.line === undefined ? "" : `, ${one.line}`}, ${state.said.toLowerCase()}`}
      onOpen={one.onOpen}
      below={
        step === undefined && tail === undefined && one.acts === undefined ? null : (
          <>
            <div className="armada-now__sub">
              {step === undefined ? null : onStep === undefined ? (
                <span className="armada-now__step">{step.name}</span>
              ) : (
                <Tooltip label="Show on the canvas">
                  <button
                    type="button"
                    className="armada-now__step"
                    aria-label={`Show ${step.name} on the canvas`}
                    aria-pressed={focused}
                    onClick={() => onStep(step.id)}
                  >
                    <Workflow size={12} strokeWidth={2} aria-hidden />
                    {step.name}
                  </button>
                </Tooltip>
              )}
              {(one.acts ?? []).map((act) => {
                const Glyph = ACT[act.glyph];
                return (
                  <Tooltip key={act.key} label={act.said}>
                    <button type="button" className="armada-now__act" aria-label={act.said} onClick={act.onAct}>
                      <Glyph size={12} strokeWidth={2} aria-hidden />
                    </button>
                  </Tooltip>
                );
              })}
              {tail === undefined ? null : (
                <Tooltip label={shown ? "Hide output" : "Show output"}>
                  <button
                    type="button"
                    className="armada-now__fold"
                    aria-label={`Output of ${one.name}`}
                    aria-expanded={shown}
                    onClick={() => setShown(!shown)}
                  >
                    {shown ? <ChevronDown size={12} strokeWidth={2} aria-hidden /> : <ChevronRight size={12} strokeWidth={2} aria-hidden />}
                  </button>
                </Tooltip>
              )}
            </div>
            {tail === undefined || !shown ? null : <pre className="armada-now__tail">{tail.join("\n")}</pre>}
          </>
        )
      }
    >
      <Kind of={one.of} />
      <span className="armada-now__text">
        {one.name}
        {one.line === undefined ? null : <span className="armada-now__line">{one.line}</span>}
      </span>
      <Tooltip label={state.said}>
        <span className="armada-now__mark" data-state={one.state} data-pulsing={one.state === "running" || undefined} role="img" aria-label={state.said}>
          <state.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </Row>
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

/**
 * One decision at a time, nothing preselected. Next goes to the next one still open;
 * the last decision carries Answer instead, which sends every pick together.
 */
function PlanAsk({ ask, onSketch }: { ask: NowPlanAsk; onSketch: (sketch: NowSketch | undefined) => void }) {
  const [at, setAt] = useState(0);
  const [picks, setPicks] = useState<Record<string, string>>({});
  const [sent, setSent] = useState(false);
  const decision = ask.decisions[at];
  const drawn = sent ? undefined : decision?.sketch;
  useEffect(() => {
    onSketch(drawn);
    return () => onSketch(undefined);
  }, [drawn, onSketch]);
  if (decision === undefined) return null;
  const last = at === ask.decisions.length - 1;
  const picked = picks[decision.id];
  const next = () => {
    const open = ask.decisions.findIndex((one, index) => index > at && picks[one.id] === undefined);
    setAt(open === -1 ? at + 1 : open);
  };
  return (
    <li className="armada-now__ask">
      {ask.decisions.length < 2 ? null : (
        <ol className="armada-now__dots" aria-label="Decisions">
          {ask.decisions.map((one, index) => (
            <li key={one.id}>
              <Tooltip label={one.question}>
                <span
                  className="armada-now__dot"
                  role="img"
                  aria-label={one.question}
                  aria-current={index === at ? "step" : undefined}
                  data-state={index === at ? "current" : picks[one.id] === undefined ? "open" : "answered"}
                />
              </Tooltip>
            </li>
          ))}
        </ol>
      )}
      <RadioGroup label={decision.question} key={decision.id}>
        {decision.options.map((option) => (
          <Radio
            key={option.id}
            name={`${ask.key}-${decision.id}`}
            value={option.id}
            checked={picked === option.id}
            disabled={sent}
            onChange={() => setPicks({ ...picks, [decision.id]: option.id })}
          >
            {option.label}
          </Radio>
        ))}
      </RadioGroup>
      {last ? (
        <Button
          size="sm"
          disabled={picked === undefined || sent}
          onClick={() => {
            setSent(true);
            ask.onAnswer(picks);
          }}
        >
          Answer
        </Button>
      ) : (
        <Button size="sm" disabled={picked === undefined} onClick={next}>
          Next
        </Button>
      )}
    </li>
  );
}
