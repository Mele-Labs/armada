// What fills the flight deck's dynamic area once something is picked: a Job as a mini canvas beside what it is doing
// now, a Session as its own miniature, a bay as the facts about it, a pull request as the facts the forge holds,
// and a call as the real call card. Every pane has one head with the way back, which Esc is too. Mock only.
// proto/cockpit-flight-deck.

import { useEffect, useMemo, useRef, type ReactNode } from "react";
import { Box, ExternalLink, GitPullRequest, Rows3, ShieldCheck, Square, SquareTerminal, X, type LucideIcon } from "lucide-react";
import { Button, NowPanel, RunNode, StepActivityMark, Tooltip, isPressed, pressedDigit } from "@armada/components";
import type { RunNodeState, StepActivity } from "@armada/components";
import { stateOf as bayStateOf } from "@armada/components/src/compositions/PoolSlots/PoolTile";
import type { WorktreeSlot } from "@armada/protocol";
import { nowPanelOf } from "@armada/jobs";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import { age, type Item } from "../../Dashboard";
import { useAnswering } from "../../cockpit/answers";
import { CallCard, type CardKeys } from "../../cockpit/CallCard";
import { dotsOf } from "../../cockpit/horizon";
import { heldFor } from "../../cockpit/metro";
import type { CockpitProps } from "../../cockpit/body-seam";
import { viewsOf } from "../../merge-line";
import { SessionMini } from "../../sessions";
import { useSessions } from "../../sessions-draft";
import { answered, leaveAutopilot, select, useDeck } from "./deck-model";
import { MarkGlyph, PrChip, StepsBar, type Acts } from "./shared";
import type { Pr, Ship, Subject } from "./ship";
import { toneOf } from "./tabs-model";

/** The one head every pane has: what it is, its name, where it is, its acts, and the way back. */
export function PaneHead({ glyph: Glyph, kind, title, where, mark, children }: { glyph: LucideIcon; kind: string; title: string; where?: string | undefined; mark?: ReactNode; children?: ReactNode }) {
  const { view, sel } = useDeck();
  return (
    <header className="deck-band deck-pane__head">
      <Tooltip label={kind}>
        <span className="deck-pane__kind" role="img" aria-label={kind}>
          <Glyph size={16} aria-hidden />
        </span>
      </Tooltip>
      {mark}
      <h2 className="deck-pane__title">{title}</h2>
      {where === undefined ? null : <span className="deck-pane__where">{where}</span>}
      <span className="deck-pane__acts">
        {children}
        <Tooltip label={`Back to the ${view}`} shortcut="Esc">
          <Button variant="ghost" size="sm" iconOnly aria-label={`Back to the ${view}`} onClick={() => (sel?.kind === "autopilot" ? leaveAutopilot() : select(undefined))}>
            <X size={16} aria-hidden />
          </Button>
        </Tooltip>
      </span>
    </header>
  );
}

/** Stop and Open, which a Job and a Session carry alike. */
function SubjectActs({ subject, acts, onPr }: { subject: Subject; acts: Acts; onPr: (pr: Pr) => void }) {
  const kind = subject.kind === "job" ? "Job" : "Session";
  return (
    <>
      {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => onPr(subject.pr!)} />}
      {subject.stoppable ? (
        <Tooltip label="Stop">
          <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={() => acts.stop(subject)}>
            <Square size={16} aria-hidden />
          </Button>
        </Tooltip>
      ) : null}
      <Tooltip label={`Open the ${kind}`}>
        <Button variant="ghost" size="sm" iconOnly aria-label={`Open the ${kind}`} onClick={() => acts.open(subject)}>
          <ExternalLink size={16} aria-hidden />
        </Button>
      </Tooltip>
    </>
  );
}

// ---- A Job ---------------------------------------------------------------------------------------------

/** Where each step of the Job's workflow stands: the ones before it done, the one it is at live, the rest not reached. */
function stepsOf(props: CockpitProps, subject: Subject) {
  const job = props.state.jobs.find((one) => one.id === subject.id);
  const steps = props.state.holds.workflows.find((one) => one.id === job?.workflow_id)?.steps ?? [];
  const at = steps.findIndex((step) => step.step_id === job?.current_step_id);
  const tone = toneOf(subject);
  return steps.map((step, index) => {
    const here = index === at;
    const state: RunNodeState = index < at ? "done" : here ? "live" : "upcoming";
    const activity: StepActivity = index < at ? "advanced" : here ? (tone === "bad" ? "failed" : tone === "ask" ? "awaiting_human" : "running") : "not_started";
    const said = index < at ? "Done" : here ? (tone === "bad" ? "Check failed" : tone === "ask" ? "Waiting on you" : "Running") : "Not started";
    return { step, state, activity, said, here };
  });
}

/**
 * A slim strip of the call's own answers, over a Job's or Session's pane, for a call whose act lives only on the call
 * card: an option, a permission, an approval. The answers are the card's, from the same hook, so answering clears it.
 */
function CallStrip({ item, props }: { item: Item; props: CockpitProps }) {
  const finish = () => answered(item.key);
  const answer = useAnswering(item, props, props.state, finish, finish);
  if (answer.answers.length === 0) return null;
  const held = answer.pending !== undefined;
  return (
    <div className="deck-callstrip" data-hue={item.hue} role="group" aria-label={`Call: ${item.title}`}>
      <span className="deck-callstrip__ask">{answer.ask}</span>
      {answer.target === undefined ? null : <code className="deck-callstrip__target">{answer.target}</code>}
      <span className="deck-callstrip__acts">
        {answer.answers.map((one) => {
          const button = (
            <Button key={one.id} size="sm" variant={one.recommended === true ? "primary" : "secondary"} pending={answer.pending === one.id} disabled={held && answer.pending !== one.id} onClick={() => one.run()}>
              {one.label}
            </Button>
          );
          return one.says === undefined ? button : (
            <Tooltip key={one.id} label={one.says}>
              {button}
            </Tooltip>
          );
        })}
      </span>
    </div>
  );
}

export function JobPane({ subject, props, acts, onPr, call }: { subject: Subject; props: CockpitProps; acts: Acts; onPr: (pr: Pr) => void; call?: Item | undefined }) {
  const run = useMemo(() => stepsOf(props, subject), [props, subject]);
  const live = useRef<HTMLLIElement>(null);
  useEffect(() => {
    live.current?.scrollIntoView?.({ block: "center" });
  }, [subject.key]);
  const nowing = nowPanelOf(props.nows?.[subject.id], { onOpenJob: props.onOpen, onSaid: () => {} });
  const tone = toneOf(subject);
  return (
    <section className="deck-pane" aria-label={`Job: ${subject.title}`}>
      <PaneHead glyph={Box} kind="Job" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone }} />}>
        <SubjectActs subject={subject} acts={acts} onPr={onPr} />
      </PaneHead>
      {call === undefined || (nowing?.asks?.length ?? 0) > 0 ? null : <CallStrip item={call} props={props} />}
      <div className="deck-job">
        <div className="deck-job__canvas" role="group" aria-label="Canvas">
          {subject.step === undefined ? null : (
            <div className="deck-job__steps">
              <StepsBar step={subject.step} tone={tone} />
            </div>
          )}
          <ol className="deck-job__run">
            {run.map(({ step, state, activity, said, here }) => (
              <li key={step.step_id} ref={here ? live : undefined} className="deck-job__node">
                <RunNode kind="step" name={step.label} id={step.step_id} activity={activity} said={said} state={state} narrow />
              </li>
            ))}
          </ol>
        </div>
        <div className="deck-job__now" role="group" aria-label="Running now">
          {nowing === undefined ? null : <NowPanel {...nowing} />}
        </div>
      </div>
    </section>
  );
}

// ---- A Session -----------------------------------------------------------------------------------------

export function SessionPane({ subject, props, acts, onPr, call }: { subject: Subject; props: CockpitProps; acts: Acts; onPr: (pr: Pr) => void; call?: Item | undefined }) {
  const tone = toneOf(subject);
  return (
    <section className="deck-pane" aria-label={`Session: ${subject.title}`}>
      <PaneHead glyph={SquareTerminal} kind="Session" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone }} />}>
        <SubjectActs subject={subject} acts={acts} onPr={onPr} />
      </PaneHead>
      {call === undefined ? null : <CallStrip item={call} props={props} />}
      <div className="deck-session">
        <SessionMini sessionId={subject.id} onOpen={props.onOpenSession} fill />
      </div>
    </section>
  );
}

// ---- Facts ---------------------------------------------------------------------------------------------

function Facts({ rows }: { rows: readonly (readonly [string, ReactNode, boolean?])[] }) {
  return (
    <dl className="deck-facts">
      {rows
        .filter(([, value]) => value !== undefined && value !== null && value !== "")
        .map(([term, value, mono]) => (
          <div key={term} className="deck-facts__row">
            <dt>{term}</dt>
            <dd data-mono={mono || undefined}>{value}</dd>
          </div>
        ))}
    </dl>
  );
}

/** A Job or Session the facts name, as a link that opens it in this same area. */
function Holder({ kind, id, title }: { kind: "job" | "session"; id: string; title: string }) {
  const Glyph = kind === "job" ? Box : SquareTerminal;
  return (
    <button type="button" className="deck-link" onClick={() => select({ kind, key: `${kind}:${id}` })}>
      <Glyph size={12} aria-hidden />
      <span>{title}</span>
    </button>
  );
}

// ---- A bay ---------------------------------------------------------------------------------------------

export function SlotPane({ slot, repo, now }: { slot: WorktreeSlot; repo: string; now: number }) {
  const state = bayStateOf({ slot });
  const held = slot.held;
  const holder = held.state === "job" ? <Holder kind="job" id={held.job_id} title={held.job_title ?? held.job_id} /> : held.state === "hosted_session" ? <Holder kind="session" id={held.session_id} title={held.session_title ?? held.session_id} /> : undefined;
  return (
    <section className="deck-pane" aria-label={`Bay: slot-${slot.slot}`}>
      <PaneHead glyph={Rows3} kind="Bay" title={`slot-${slot.slot}`} where={repo} mark={<MarkGlyph mark={{ Glyph: state.Glyph, said: state.said, tone: slot.closed === true || state.bay === "stranded" ? "bad" : "calm" }} />} />
      <div className="deck-pane__body">
        <Facts
          rows={[
            ["State", slot.closed === true ? "Closed" : state.word],
            ["Holder", holder],
            ["Branch", slot.branch, true],
            ["Held for", heldFor(slot.since, now)],
            ["Base", slot.base, true],
            ["Behind", (slot.behind ?? 0) > 0 ? `${slot.behind} commits` : undefined],
            ["Checkout", slot.warm ? "Warm" : "Cold"],
            ["Path", slot.path, true],
          ]}
        />
      </div>
    </section>
  );
}

// ---- A pull request ------------------------------------------------------------------------------------

export function PrPane({ pr, owner, props, acts }: { pr: Pr; owner: Subject | undefined; props: CockpitProps; acts: Acts }) {
  const sessions = useSessions();
  const views = viewsOf(props.state);
  const view = views.find((one) => one.root === pr.root);
  const dot = view === undefined ? undefined : dotsOf(view, props.state.mergeLines, views, sessions).find((one) => one.key === `pull:${pr.number}`);
  const card = dot?.card;
  const opened = heldFor(card?.opened, props.now);
  const who = owner === undefined || owner.kind === "pr" ? undefined : <Holder kind={owner.kind} id={owner.id} title={owner.title} />;
  return (
    <section className="deck-pane" aria-label={`Pull request #${pr.number}`}>
      <PaneHead glyph={GitPullRequest} kind="Pull request" title={`#${pr.number} ${pr.title}`} where={view === undefined ? undefined : props.state.holds.repositories?.find((one) => one.root === pr.root)?.manifest?.repository}>
        {owner !== undefined && owner.mergeable ? (
          <Button variant="secondary" size="sm" aria-label={`Merge #${pr.number}`} onClick={() => acts.merge(owner)}>
            Merge
          </Button>
        ) : null}
        <Tooltip label="Open on the forge">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open on the forge" onClick={() => props.onOpenLink(pr.url)}>
            <ExternalLink size={16} aria-hidden />
          </Button>
        </Tooltip>
      </PaneHead>
      <div className="deck-pane__body" data-tone={pr.stage === "red" ? "bad" : undefined}>
        <Facts
          rows={[
            ["State", card?.says],
            ["Checks", card?.check],
            ["Place", card?.place === undefined ? undefined : String(card.place)],
            ["Branch", pr.branch, true],
            ["Author", pr.author],
            ["Opened", opened === undefined ? undefined : `${opened} ago`],
            ["Review", card?.review],
            ["Draft", card?.draft === true ? "Yes" : undefined],
            ["Conflicts", card?.conflicts === true ? "Yes" : undefined],
            ["Owner", who],
          ]}
        />
      </div>
    </section>
  );
}

// ---- A call --------------------------------------------------------------------------------------------

/** The real call card, answered through the acts the Cockpit's is. The keys it answers to are bound while it is up. */
export function CallPane({ item, props }: { item: Item; props: CockpitProps }) {
  const answering = useRef<CardKeys | undefined>(undefined);
  const finish = () => {
    answered(item.key);
    select(undefined);
  };
  const later = () => select(undefined);
  useListKeydown((event) => {
    if (event.repeat) return;
    const bare = !(event.metaKey || event.ctrlKey || event.altKey);
    if (holdsText(event.target)) {
      const field = event.target;
      if (field instanceof HTMLInputElement && field.value === "" && (isPressed("close", event) || (bare && event.key === "ArrowDown"))) {
        event.preventDefault();
        field.blur();
      }
      return;
    }
    const card = answering.current;
    if (card === undefined) return;
    const claim = () => event.preventDefault();
    const digit = pressedDigit("call_pick", event);
    if (digit !== null) return claim(), card.pickNumber(digit);
    if (card.pickKey(event)) return claim();
    if (isPressed("call_reply", event) && card.change !== undefined) return claim(), card.change();
    if (isPressed("call_reply", event)) {
      const field = document.querySelector<HTMLTextAreaElement>(".deck-call .armada-callcard__reply textarea");
      return field === null ? undefined : (claim(), field.focus());
    }
    if (isPressed("call_expand", event)) return card.expand === undefined ? undefined : (claim(), card.expand());
    if (isPressed("call_send", event)) {
      if ((event.target as HTMLElement).tagName === "BUTTON" && document.querySelector(".deck-call .armada-callcard")?.contains(event.target as Node) === true) return;
      return card.canSend ? (claim(), card.send()) : undefined;
    }
    if (isPressed("call_later", event) || isPressed("close", event)) return claim(), card.pending === undefined ? later() : undefined;
    if (isPressed("call_dismiss", event)) return claim(), card.dismiss();
    if (isPressed("call_open_walk", event)) return card.openWalk === undefined ? undefined : (claim(), card.openWalk());
    if (isPressed("open", event)) return card.open === undefined ? undefined : (claim(), card.open());
  });
  const jobId = item.job?.id;
  const nowing = jobId === undefined ? undefined : nowPanelOf(props.nows?.[jobId], { onOpenJob: props.onOpen, onSaid: () => {} });
  const Icon = item.icon;
  return (
    <section className="deck-pane" aria-label={`Call: ${item.title}`} data-hue={item.hue}>
      <PaneHead glyph={Icon} kind={item.kind} title={item.title} where={[item.where, age(item.at, props.now)].filter((one) => one !== "").join(" · ")} />
      <div className="deck-call armada-cockpit__scrim">
        <div className="armada-stack" data-consoled style={{ ["--behind" as string]: 0 }}>
          <CallCard key={item.key} item={item} now={props.now} nowing={nowing} state={props.state} hosts={props} finish={finish} later={later} dismiss={finish} leaving={undefined} from={undefined} answering={answering} />
        </div>
      </div>
    </section>
  );
}

// ---- Checks --------------------------------------------------------------------------------------------

type CheckRow = { key: string; name: string; where: string; state: "running" | "passed" | "failed"; open: () => void };

/** The Checks the ship has going or just done: those a Job's Drone is running, and a pull request's ci while it runs. */
function checksOf(props: CockpitProps, ship: Ship): CheckRow[] {
  const ofJobs = ship.subjects.flatMap((subject) =>
    subject.kind !== "job"
      ? []
      : (props.nows?.[subject.id]?.running ?? []).flatMap((one) => (one.of === "check" ? [{ key: `${subject.key}:${one.key}`, name: one.name, where: subject.title, state: one.state, open: () => select({ kind: "job" as const, key: subject.key }) }] : [])),
  );
  const ofPrs = ship.prs.filter((pr) => pr.stage === "checking").map((pr): CheckRow => ({ key: `${pr.key}:ci`, name: "ci", where: `#${pr.number} ${pr.title}`, state: "running", open: () => select({ kind: "pr", key: pr.key }) }));
  return [...ofJobs, ...ofPrs];
}

const CHECK_SAID = { running: "Running", passed: "Passed", failed: "Failed" } as const;
const CHECK_ACTIVITY = { running: "running", passed: "advanced", failed: "failed" } as const;

export function ChecksPane({ props, ship }: { props: CockpitProps; ship: Ship }) {
  const rows = checksOf(props, ship);
  const group = (name: string, list: readonly CheckRow[]) =>
    list.length === 0 ? null : (
      <section className="deck-checks__group" aria-label={name}>
        <header className="deck-band">{name}</header>
        <ul className="deck-checks__rows">
          {list.map((one) => (
            <li key={one.key}>
              <button type="button" className="deck-check" data-state={one.state} aria-label={`${one.name}, ${one.where}, ${CHECK_SAID[one.state]}`} onClick={one.open}>
                <StepActivityMark activity={CHECK_ACTIVITY[one.state]} label={CHECK_SAID[one.state]} says={`${one.name}, ${CHECK_SAID[one.state].toLowerCase()}`} pulsing={one.state === "running"} />
                <span className="deck-check__name">{one.name}</span>
                <span className="deck-check__where">{one.where}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>
    );
  return (
    <section className="deck-pane" aria-label="Checks">
      <PaneHead glyph={ShieldCheck} kind="Checks" title="Checks" />
      <div className="deck-checks">
        {group("Running", rows.filter((one) => one.state === "running"))}
        {group("Finished", rows.filter((one) => one.state !== "running"))}
      </div>
    </section>
  );
}
