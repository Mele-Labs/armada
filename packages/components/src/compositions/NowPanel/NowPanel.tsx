import { Fragment, useEffect, useMemo, useState, type KeyboardEvent, type ReactNode } from "react";
import { Activity, Bot, Box, ChevronDown, ChevronRight, CircleDot, Compass, CornerUpRight, Cpu, DraftingCompass, MessageSquare, Network, OctagonAlert, PanelRightClose, RotateCw, Scale, ShieldCheck, ShieldOff, ShieldX, SkipForward, Waypoints, Workflow } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { WhoMark } from "../WhoMark/WhoMark";
import type { NowAsker } from "../AskerView/AskerView";
import { AskActs, AskAnswer, AskAnswers, AskCard, AskChoices, AskPips, AskQuestion } from "../AskCard/AskCard";
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
 * A sketch the asking Drone drew to go with its question: a scene, which the Overview validates
 * and draws (`SketchScene`). It is handed over as it arrived, so a bad one draws nothing.
 */
export type NowSketch = { scene: unknown };

/** What the Overview is asked to draw: the sketch, and the current-state one it changes, where there is one. */
export type NowSketchShow = { sketch: NowSketch; against?: NowSketch };

/** Something the owner asked about a part of a sketch, kept under the ask it was asked of. */
export type NowRemark = { key: string; about: string; said: string };

/** What the Overview shows to the left of the panel while something asks: the sketch, the asker, or the canvas. */
export type NowSketchView = "sketch" | "asker" | "canvas";

/** A Plan decision the Drone needs made, with the answers it takes. Mock data until Fleet has a plan interview. */
export type NowDecision = {
  id: string;
  question: string;
  /** An option's `sketch` is how the end result looks if it is taken. */
  options: readonly { id: string; label: string; sketch?: NowSketch }[];
  /** The current state, shown while no option is hovered, focused or picked. */
  sketch?: NowSketch;
};

/** Decisions are asked one at a time. The answer goes once, at the last, as an option id per decision. */
export type NowPlanAsk = {
  key: string;
  kind: "plan";
  decisions: readonly NowDecision[];
  /** What the Plan Drone is doing and has changed, drawn on the left while no sketch is. */
  asker?: NowAsker;
  /** `edits` is what the owner marked on the sketches, where he marked any, so the Drone reads both. */
  onAnswer: (answers: Readonly<Record<string, string>>, edits?: Readonly<Record<string, unknown>>) => void;
};

/** A Judge's or a Drone's question, opened where it is answered. */
export type NowOpenAsk = {
  key: string;
  kind: "judge" | "drone";
  name: string;
  text: string;
  /** Shown beside the canvas while the question is open. */
  sketch?: NowSketch;
  /** The asker's live output, what it changed, and for a Judge the product and checks. */
  asker?: NowAsker;
  onOpen: () => void;
};

/**
 * What the Job's Commander cannot do itself, asked once as one card: the fact that stopped it, whose
 * act the fix is, and the answers it takes. **The answer is the owner's act**, never the Commander's:
 * a Check is skipped or edited by a person, and the Commander is woken by the answer.
 */
export type NowCommanderAsk = {
  key: string;
  kind: "commander";
  /** The fact, bare. */
  text: string;
  /** Whose act the fix is, said as a fact. */
  reach: string;
  question: string;
  options: readonly { id: string; label: string }[];
  /** Sent once. A host that has Fleet's word answers `false` where it refused, and the card takes another pick. */
  onAnswer: (optionId: string) => void | Promise<boolean>;
};

/**
 * Whatever a Job held under its lead that a person answers: the approval, the review gate, a Drone's
 * question, a held command, a Trigger's alert. **The host draws what is inside**, since each is an
 * act of its own; this draws the card they stand in, the Cockpit's own (`AskCard`).
 */
export type NowSlotAsk = {
  key: string;
  kind: "slot";
  /** The band's word: `Review`, `Drone question`. */
  band: string;
  icon: LucideIcon;
  /** What it is about, over the body. */
  title: string;
  /** How long it has waited, already worded. */
  age?: string;
  children: ReactNode;
};

export type NowAsk = NowPlanAsk | NowOpenAsk | NowCommanderAsk | NowSlotAsk;

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
  /** Whose hand is on the task it works, where that is the Commander's: a mark beside the name. */
  by?: { who: "commander"; said?: string };
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
  onSketch?: (show: NowSketchShow | undefined) => void;
  /** What was asked about the sketch's parts, drawn under the asks. Absent draws none. */
  thread?: readonly NowRemark[];
  /** What the owner marked on the sketches, sent with a Plan answer. The host keeps it. */
  edits?: Readonly<Record<string, unknown>>;
  /** Told the asker of the ask now open, and `undefined` once none is. Absent leaves askers undrawn. */
  onAsker?: (asker: NowAsker | undefined) => void;
  /** What a press on a file in the asker view does. The panel draws no file; the host passes it on. */
  onOpenFile?: (path: string) => void;
  /** Which the Overview shows while something asks. Absent reads as the sketch, else the asker. */
  sketchView?: NowSketchView;
  /** The head's switch between what applies: Sketch, Asker, Canvas. Absent draws none. */
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

const ASK_ORDER = ["slot", "plan", "commander", "judge", "drone"] as const;
const RUN_ORDER = ["drone", "check", "judge"] as const;

export function NowPanel({ asks = [], issues = [], running = [], waiting = [], onSkipAll, onHide, onStep, focusedStep, onSketch, onAsker, sketchView, onSketchView, thread = [], edits }: NowPanelProps) {
  // The plan decision asked now reports its own sketch; a Judge's or Drone's is read off the asks.
  const [planSketch, setPlanSketch] = useState<NowSketchShow | undefined>(undefined);
  const openSketch = asks.flatMap((ask) => (ask.kind === "plan" || ask.kind === "commander" || ask.kind === "slot" || ask.sketch === undefined ? [] : [ask.sketch]))[0];
  const openShow = useMemo<NowSketchShow | undefined>(() => (openSketch === undefined ? undefined : { sketch: openSketch }), [openSketch]);
  const asked = planSketch ?? openShow;
  const [planAsker, setPlanAsker] = useState<NowAsker | undefined>(undefined);
  const openAsker = asks.flatMap((ask) => (ask.kind === "plan" || ask.kind === "commander" || ask.kind === "slot" || ask.asker === undefined ? [] : [ask.asker]))[0];
  const askerNow = planAsker ?? openAsker;
  const shownView = sketchView ?? (asked !== undefined ? "sketch" : "asker");
  useEffect(() => {
    onAsker?.(askerNow);
    return () => onAsker?.(undefined);
  }, [askerNow, onAsker]);
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
        {(asked === undefined && askerNow === undefined) || onSketchView === undefined ? null : (
          <div className="armada-now__switch" role="group" aria-label="Show on the left">
            {asked === undefined ? null : (
              <Tooltip label="Sketch">
                <button type="button" className="armada-now__switch-act" aria-label="Sketch" aria-pressed={shownView === "sketch"} onClick={() => onSketchView("sketch")}>
                  <DraftingCompass size={16} strokeWidth={2} aria-hidden />
                </button>
              </Tooltip>
            )}
            {askerNow === undefined ? null : (
              <Tooltip label="Asker">
                <button type="button" className="armada-now__switch-act" aria-label="Asker" aria-pressed={shownView === "asker"} onClick={() => onSketchView("asker")}>
                  <Activity size={16} strokeWidth={2} aria-hidden />
                </button>
              </Tooltip>
            )}
            <Tooltip label="Canvas">
              <button type="button" className="armada-now__switch-act" aria-label="Canvas" aria-pressed={shownView === "canvas"} onClick={() => onSketchView("canvas")}>
                <Network size={16} strokeWidth={2} aria-hidden />
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
        // **Each ask is the Cockpit's call card**, so it looks the same here as where it is also asked.
        <div className="armada-now__cards" role="group" aria-label="Asks you">
          {ASK_ORDER.map((kind) => {
            const of = asks.filter((ask) => ask.kind === kind);
            return of.length === 0 ? null : (
              <Fragment key={kind}>
                {of.map((ask) =>
                  ask.kind === "plan" ? (
                    <PlanAsk key={ask.key} ask={ask} onSketch={setPlanSketch} onAsker={setPlanAsker} {...(edits === undefined ? {} : { edits })} />
                  ) : ask.kind === "commander" ? (
                    <CommanderAsk key={ask.key} ask={ask} />
                  ) : ask.kind === "slot" ? (
                    <AskCard key={ask.key} flow icon={ask.icon} kind={ask.band} title={ask.title} {...(ask.age === undefined ? {} : { age: ask.age })}>
                      {ask.children}
                    </AskCard>
                  ) : (
                    <AskRow key={ask.key} ask={ask} />
                  ),
                )}
              </Fragment>
            );
          })}
          {thread.length === 0 ? null : (
            <ul className="armada-now__rows">
              {thread.map((one) => (
                <li key={one.key} className="armada-now__remark">
                  <span className="armada-now__remark-about">{one.about}</span>
                  <span className="armada-now__text">{one.said}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
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
      {one.by === undefined ? null : <WhoMark who={one.by.who} {...(one.by.said === undefined ? {} : { said: one.by.said })} />}
      <Tooltip label={state.said}>
        <span className="armada-now__mark" data-state={one.state} data-pulsing={one.state === "running" || undefined} role="img" aria-label={state.said}>
          <state.Glyph size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </Row>
  );
}

/** What the digit keys do on a card's answers: the number on a tile picks it, as it does in the Cockpit. */
function picksByDigit(count: number, pick: (index: number) => void) {
  return (event: KeyboardEvent<HTMLDivElement>) => {
    const digit = Number(event.key);
    if (!Number.isInteger(digit) || digit < 1 || digit > count) return;
    event.preventDefault();
    pick(digit - 1);
  };
}

/** The Commander's one card: the fact, whose act the fix is, and the answers, sent together. */
function CommanderAsk({ ask }: { ask: NowCommanderAsk }) {
  const [picked, setPicked] = useState<number | undefined>(undefined);
  const [sent, setSent] = useState(false);
  return (
    <AskCard flow icon={Compass} kind="Commander asks" title={ask.text}>
      <p className="armada-callcard__origin">{ask.reach}</p>
      <AskQuestion>{ask.question}</AskQuestion>
      <AskChoices label={ask.question} onKeyDown={picksByDigit(ask.options.length, (index) => !sent && setPicked(index))}>
        <AskAnswers>
          {ask.options.map((option, index) => (
            <AskAnswer key={option.id} one={{ label: option.label }} mark={String(index + 1)} picked={picked === index} held={sent} onPick={() => setPicked(index)} />
          ))}
        </AskAnswers>
      </AskChoices>
      <AskActs>
        <Button
          variant="primary"
          disabled={picked === undefined || sent}
          onClick={() => {
            const one = picked === undefined ? undefined : ask.options[picked];
            if (one === undefined) return;
            setSent(true);
            void Promise.resolve(ask.onAnswer(one.id)).then((accepted) => accepted === false && setSent(false));
          }}
        >
          Answer
        </Button>
      </AskActs>
    </AskCard>
  );
}

/** A Judge's or a Drone's question: what it asks, and the press that opens it where it is answered. */
function AskRow({ ask }: { ask: NowOpenAsk }) {
  return (
    <AskCard flow icon={ask.kind === "judge" ? Scale : Bot} kind={ask.kind === "judge" ? "Judge question" : "Drone question"} title={ask.text}>
      <p className="armada-callcard__origin">{ask.name}</p>
      <AskActs>
        <Button variant="primary" aria-label={`Open ${ask.name}, asks you`} onClick={ask.onOpen}>
          Open
        </Button>
      </AskActs>
    </AskCard>
  );
}

/**
 * One decision at a time, nothing preselected. Next goes to the next one still open;
 * the last decision carries Answer instead, which sends every pick together.
 */
function PlanAsk({ ask, onSketch, onAsker, edits }: { ask: NowPlanAsk; onSketch: (show: NowSketchShow | undefined) => void; onAsker: (asker: NowAsker | undefined) => void; edits?: Readonly<Record<string, unknown>> }) {
  const [at, setAt] = useState(0);
  const [picks, setPicks] = useState<Record<string, string>>({});
  const [sent, setSent] = useState(false);
  // **An option previews its own sketch** while hovered or focused, and keeps it once picked; with
  // none of those the decision's own sketch stands, which is how things are now.
  const [hovered, setHovered] = useState<string | undefined>(undefined);
  const [focused, setFocused] = useState<string | undefined>(undefined);
  const decision = ask.decisions[at];
  const optionSketch = (id: string | undefined) => decision?.options.find((one) => one.id === id)?.sketch;
  const previewed = optionSketch(hovered) ?? optionSketch(focused) ?? optionSketch(decision === undefined ? undefined : picks[decision.id]);
  // **An option's sketch is read against the decision's own**, which is how things are now.
  const own = decision?.sketch;
  const drawn = useMemo<NowSketchShow | undefined>(
    () => (sent ? undefined : previewed !== undefined ? { sketch: previewed, ...(own === undefined ? {} : { against: own }) } : own === undefined ? undefined : { sketch: own }),
    [sent, previewed, own],
  );
  useEffect(() => {
    onSketch(drawn);
    return () => onSketch(undefined);
  }, [drawn, onSketch]);
  const asker = sent ? undefined : ask.asker;
  useEffect(() => {
    onAsker(asker);
    return () => onAsker(undefined);
  }, [asker, onAsker]);
  if (decision === undefined) return null;
  const last = at === ask.decisions.length - 1;
  const picked = picks[decision.id];
  const pick = (id: string) => !sent && setPicks({ ...picks, [decision.id]: id });
  const next = () => {
    const open = ask.decisions.findIndex((one, index) => index > at && picks[one.id] === undefined);
    setHovered(undefined);
    setFocused(undefined);
    setAt(open === -1 ? at + 1 : open);
  };
  return (
    <AskCard flow icon={MessageSquare} kind="Plan question">
      <AskQuestion>
        {decision.question}
        {ask.decisions.length < 2 ? null : <AskPips decisions={ask.decisions} at={at} />}
      </AskQuestion>
      <AskChoices key={decision.id} label={decision.question} onKeyDown={picksByDigit(decision.options.length, (index) => pick(decision.options[index]!.id))}>
        <AskAnswers>
          {decision.options.map((option, index) => (
            <AskAnswer
              key={option.id}
              one={{ label: option.label }}
              mark={String(index + 1)}
              picked={picked === option.id}
              held={sent}
              onPick={() => pick(option.id)}
              onHover={(on) => setHovered(on ? option.id : undefined)}
              onFocusing={(on) => setFocused(on ? option.id : undefined)}
            />
          ))}
        </AskAnswers>
      </AskChoices>
      <AskActs>
        {last ? (
          <Button
            variant="primary"
            disabled={picked === undefined || sent}
            onClick={() => {
              setSent(true);
              // A caller with nothing marked sees the one-argument call it always has.
              if (edits === undefined) ask.onAnswer(picks);
              else ask.onAnswer(picks, edits);
            }}
          >
            Answer
          </Button>
        ) : (
          <Button variant="primary" disabled={picked === undefined} onClick={next}>
            Next
          </Button>
        )}
      </AskActs>
    </AskCard>
  );
}
