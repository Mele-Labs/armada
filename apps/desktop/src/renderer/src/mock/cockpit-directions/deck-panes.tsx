// What fills the flight deck's dynamic area once something is picked: a Job as a mini canvas beside what it is doing
// now, a Session as its own miniature, a bay as the facts about it, a pull request as the facts the forge holds,
// and a call as the real call card. Every pane has one head with the way back, which Esc is too. Mock only.
// proto/cockpit-three-directions.

import { useEffect, useMemo, useRef, type ReactNode } from "react";
import { ExternalLink, Square, X, type LucideIcon } from "lucide-react";
import { Box, SquareTerminal } from "lucide-react";
import { Button, Kbd, NowPanel, RunNode, Tooltip, isPressed, pressedDigit } from "@armada/components";
import type { RunNodeState, StepActivity } from "@armada/components";
import type { WorktreeSlot } from "@armada/protocol";
import { nowPanelOf } from "@armada/jobs";
import { holdsText } from "@armada/screens/src/keys";
import { useListKeydown } from "@armada/screens/src/list-keyboard";

import { age, type Item } from "../../Dashboard";
import { CallCard, type CardKeys } from "../../cockpit/CallCard";
import { dotsOf } from "../../cockpit/horizon";
import { heldFor } from "../../cockpit/metro";
import type { CockpitProps } from "../../cockpit/body-seam";
import { viewsOf } from "../../merge-line";
import { SessionMini } from "../../sessions";
import { useSessions } from "../../sessions-draft";
import { answered, select, useDeck } from "./deck-model";
import { MarkGlyph, PrChip, StepsBar, type Acts } from "./shared";
import type { Pr, Ship, Subject } from "./ship";
import { toneOf } from "./tabs-model";

/** The one head every pane has: what it is, its name, where it is, its acts, and the way back. */
export function PaneHead({ glyph: Glyph, kind, title, where, mark, children }: { glyph: LucideIcon; kind: string; title: string; where?: string | undefined; mark?: ReactNode; children?: ReactNode }) {
  const { view } = useDeck();
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
          <Button variant="ghost" size="sm" iconOnly aria-label={`Back to the ${view}`} onClick={() => select(undefined)}>
            <X size={16} aria-hidden />
          </Button>
        </Tooltip>
      </span>
    </header>
  );
}

// ---- A Job ---------------------------------------------------------------------------------------------

/** Where each step of the Job's workflow stands: the ones before it done, the one it is at live, the rest not reached. */
function stepsOf(props: CockpitProps, subject: Subject) {
  const job = props.state.jobs.find((one) => one.id === subject.id);
  const steps = props.state.holds.workflows.find((one) => one.id === job?.workflow_id)?.steps ?? [];
  const at = steps.findIndex((step) => step.step_id === job?.current_step_id);
  return steps.map((step, index) => {
    const tone = toneOf(subject);
    const state: RunNodeState = index < at ? "done" : index === at ? "live" : "upcoming";
    const activity: StepActivity = index < at ? "advanced" : index === at ? (tone === "bad" ? "failed" : tone === "ask" ? "awaiting_human" : "running") : "not_started";
    const said = index < at ? "Done" : index === at ? (tone === "bad" ? "Check failed" : tone === "ask" ? "Waiting on you" : "Running") : "Not started";
    return { step, state, activity, said, live: index === at };
  });
}

export function JobPane({ subject, props, acts, onPr }: { subject: Subject; props: CockpitProps; acts: Acts; onPr: (pr: Pr) => void }) {
  const run = useMemo(() => stepsOf(props, subject), [props, subject]);
  const live = useRef<HTMLLIElement>(null);
  useEffect(() => live.current?.scrollIntoView({ block: "center" }), [subject.key]);
  const nowing = nowPanelOf(props.nows?.[subject.id], { onOpenJob: props.onOpen, onSaid: () => {} });
  const tone = toneOf(subject);
  return (
    <section className="deck-pane" aria-label={`Job: ${subject.title}`}>
      <PaneHead glyph={Box} kind="Job" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone }} />}>
        {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => onPr(subject.pr!)} />}
        {subject.stoppable ? (
          <Tooltip label="Stop" shortcut="s">
            <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={() => acts.stop(subject)}>
              <Square size={16} aria-hidden />
            </Button>
          </Tooltip>
        ) : null}
        <Tooltip label="Open the Job" shortcut="↵">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open the Job" onClick={() => acts.open(subject)}>
            <ExternalLink size={16} aria-hidden />
          </Button>
        </Tooltip>
      </PaneHead>
      <div className="deck-job">
        <div className="deck-job__canvas deck-ruled" role="group" aria-label="Canvas">
          {subject.step === undefined ? null : (
            <div className="deck-job__steps">
              <StepsBar step={subject.step} tone={tone} />
            </div>
          )}
          <ol className="deck-job__run">
            {run.map(({ step, state, activity, said, live: here }) => (
              <li key={step.step_id} ref={here ? live : undefined} className="deck-job__node">
                <RunNode kind="step" name={step.label} id={step.step_id} activity={activity} said={said} state={state} narrow />
              </li>
            ))}
          </ol>
        </div>
        <div className="deck-job__now">{nowing === undefined ? null : <NowPanel {...nowing} />}</div>
      </div>
    </section>
  );
}

// ---- A Session -----------------------------------------------------------------------------------------

export function SessionPane({ subject, props, acts, onPr }: { subject: Subject; props: CockpitProps; acts: Acts; onPr: (pr: Pr) => void }) {
  const tone = toneOf(subject);
  return (
    <section className="deck-pane" aria-label={`Session: ${subject.title}`}>
      <PaneHead glyph={SquareTerminal} kind="Session" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone }} />}>
        {subject.pr === undefined ? null : <PrChip pr={subject.pr} onOpen={() => onPr(subject.pr!)} />}
        {subject.stoppable ? (
          <Tooltip label="Stop" shortcut="s">
            <Button variant="ghost" size="sm" iconOnly aria-label="Stop" onClick={() => acts.stop(subject)}>
              <Square size={16} aria-hidden />
            </Button>
          </Tooltip>
        ) : null}
        <Tooltip label="Open the Session" shortcut="↵">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open the Session" onClick={() => acts.open(subject)}>
            <ExternalLink size={16} aria-hidden />
          </Button>
        </Tooltip>
      </PaneHead>
      <div className="deck-session">
        <SessionMini sessionId={subject.id} onOpen={props.onOpenSession} fill />
      </div>
    </section>
  );
}

// ---- A bay ---------------------------------------------------------------------------------------------

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

export function SlotPane({ slot, props, ship }: { slot: WorktreeSlot; props: CockpitProps; ship: Ship }) {
  const bay = ship.bays.find((one) => one.slot === slot.slot && ship.repos.some((repo) => repo.root === one.root && one.root === ship.bays.find((b) => b === one)?.root) && one.repo === (ship.repos.find((repo) => repo.root === one.root)?.name ?? one.repo) && sameRepo(one.root, slot, props));
  const holder = bay?.holder;
  const state = slot.closed === true ? "Closed" : slot.held.state === "free" ? "Free" : holder !== undefined ? "Held" : slot.held.state === "stranded" ? "Stranded" : "Held";
  const holds = holder === undefined ? undefined : (
    <button type="button" className="deck-link" onClick={() => select({ kind: holder.kind === "job" ? "job" : "session", key: `${holder.kind}:${holder.id}` })}>
      {holder.kind === "job" ? <Box size={12} aria-hidden /> : <SquareTerminal size={12} aria-hidden />}
      <span>{holder.title}</span>
    </button>
  );
  return (
    <section className="deck-pane" aria-label={`Bay: slot-${slot.slot}`}>
      <PaneHead glyph={Box} kind="Bay" title={`slot-${slot.slot}`} where={bay?.repo ?? slot.manifest_id} mark={<span className="deck-pane__state">{state}</span>} />
      <div className="deck-pane__body">
        <Facts
          rows={[
            ["Holder", holds],
            ["Branch", slot.branch, true],
            ["Held for", heldFor(slot.since, props.now)],
            ["Path", slot.path, true],
            ["Base", slot.base, true],
            ["Behind", slot.behind > 0 ? `${slot.behind} commits` : undefined],
            ["Checkout", slot.warm ? "Warm" : "Cold"],
          ]}
        />
      </div>
    </section>
  );
}

/** Whether a bay of the ship is this slot: the same number in the same repository. */
const sameRepo = (root: string, slot: WorktreeSlot, props: CockpitProps) => props.state.holds.repositories?.find((one) => one.root === root)?.manifest?.id === slot.manifest_id;

// ---- A pull request ------------------------------------------------------------------------------------

export function PrPane({ pr, owner, props, acts }: { pr: Pr; owner: Subject | undefined; props: CockpitProps; acts: Acts }) {
  const sessions = useSessions();
  const view = viewsOf(props.state).find((one) => one.root === pr.root);
  const dot = view === undefined ? undefined : dotsOf(view, props.state.mergeLines, viewsOf(props.state), sessions).find((one) => one.key === `pull:${pr.number}`);
  const card = dot?.card;
  const opened = card?.opened === undefined ? undefined : heldFor(card.opened, props.now);
  const who = owner === undefined || owner.kind === "pr" ? undefined : (
    <button type="button" className="deck-link" onClick={() => select({ kind: owner.kind === "job" ? "job" : "session", key: owner.key })}>
      {owner.kind === "job" ? <Box size={12} aria-hidden /> : <SquareTerminal size={12} aria-hidden />}
      <span>{owner.title}</span>
    </button>
  );
  const toneName = pr.stage === "red" ? "red" : undefined;
  return (
    <section className="deck-pane" aria-label={`Pull request #${pr.number}`}>
      <PaneHead glyph={ExternalLink} kind="Pull request" title={`#${pr.number} ${pr.title}`} where={view === undefined ? undefined : props.state.holds.repositories?.find((one) => one.root === pr.root)?.manifest?.repository}>
        {owner !== undefined && owner.mergeable ? (
          <Button variant="secondary" size="sm" aria-label={`Merge #${pr.number}`} onClick={() => acts.merge(owner)}>
            Merge
            <Kbd>m</Kbd>
          </Button>
        ) : null}
        <Tooltip label="Open on the forge">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open on the forge" onClick={() => props.onOpenLink(pr.url)}>
            <ExternalLink size={16} aria-hidden />
          </Button>
        </Tooltip>
      </PaneHead>
      <div className="deck-pane__body" data-tone={toneName}>
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

/** The real call card, answered through the same acts the Cockpit's is. The keys the card answers to are bound while it is up. */
export function CallPane({ item, props, onLater }: { item: Item; props: CockpitProps; onLater: () => void }) {
  const answering = useRef<CardKeys | undefined>(undefined);
  const finish = () => {
    answered(item.key);
    select(undefined);
  };
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
      const field = document.querySelector<HTMLTextAreaElement>(".armada-callcard__reply textarea");
      return field === null ? undefined : (claim(), field.focus());
    }
    if (isPressed("call_expand", event)) return card.expand === undefined ? undefined : (claim(), card.expand());
    if (isPressed("call_send", event)) {
      if ((event.target as HTMLElement).tagName === "BUTTON" && document.querySelector(".armada-callcard")?.contains(event.target as Node) === true) return;
      return card.canSend ? (claim(), card.send()) : undefined;
    }
    if (isPressed("call_later", event) || isPressed("close", event)) return claim(), card.pending === undefined ? onLater() : undefined;
    if (isPressed("call_dismiss", event)) return claim(), card.dismiss();
    if (isPressed("call_open_walk", event)) return card.openWalk === undefined ? undefined : (claim(), card.openWalk());
    if (isPressed("open", event)) return card.open === undefined ? undefined : (claim(), card.open());
  });
  const jobId = item.job?.id ?? (item.ownedBy?.kind === "job" ? item.ownedBy.id : undefined);
  const nowing = jobId === undefined ? undefined : nowPanelOf(props.nows?.[jobId], { onOpenJob: props.onOpen, onSaid: () => {} });
  const Icon = item.icon;
  return (
    <section className="deck-pane" aria-label={`Call: ${item.title}`} data-hue={item.hue}>
      <PaneHead glyph={Icon} kind={item.kind} title={item.title} where={`${item.where} · ${age(item.at, props.now)}`} />
      <div className="deck-call armada-cockpit__scrim">
        <div className="armada-stack" data-consoled style={{ ["--behind" as string]: 0 }}>
          <CallCard key={item.key} item={item} now={props.now} nowing={nowing} state={props.state} hosts={props} finish={finish} later={onLater} dismiss={finish} leaving={undefined} from={undefined} answering={answering} />
        </div>
      </div>
    </section>
  );
}
