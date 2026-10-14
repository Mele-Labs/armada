// What fills the flight deck's dynamic area once something is picked: a Job as a mini canvas beside what it is doing
// now, a Session as its own miniature, a bay as the facts about it, a pull request as the facts the forge holds,
// and a call as the real call card. Every pane has one head with the way back, which Esc is too. Mock only.
// proto/cockpit-flight-deck.

import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Box, ExternalLink, GitPullRequest, Rows3, Square, SquareTerminal, X, type LucideIcon } from "lucide-react";
import { Button, NowPanel, Tabs, Tooltip, isPressed, pressedDigit } from "@armada/components";
import { ScrapConfirm, SlotFinding } from "@armada/components/src/compositions/PoolSlots/SlotFinding";
import { clockOf, shortOf, spanOf, type BayView, type Holder as BayHolder } from "./deck-bays";
import type { JobDetail } from "@armada/protocol";
import { JobRun, detailOf, nowPanelOf } from "@armada/jobs";
import { droneViewsOf, type DroneView } from "@armada/jobs/draft/drone";
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
import { answered, leaveAutopilot, select, setJobView, useDeck } from "./deck-model";
import { MarkGlyph, PrChip, StepsBar, type Acts } from "./shared";
import type { Pr, Subject } from "./ship";
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
      <Tooltip label={`Open the ${kind} page`}>
        <Button variant="ghost" size="sm" iconOnly aria-label={`Open the ${kind} page`} onClick={() => acts.open(subject)}>
          <ExternalLink size={16} aria-hidden />
        </Button>
      </Tooltip>
    </>
  );
}

// ---- A Job ---------------------------------------------------------------------------------------------

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

/** What the Job's run is, read from the Job Bridge is watching; the pane asks Fleet to watch it. Both the mission and the canvas draw from it. */
export type Run = { whole: JobDetail | null; drones: DroneView[] };

function useRun(subject: Subject, props: CockpitProps): Run {
  const { watched, jobDrones } = props.state;
  const whole = detailOf(watched, subject.id);
  const watching = watched.state === "read" ? watched.jobId : undefined;
  // Fleet watches one Job. Say which one this is, and say it again if the page or another pane moved it.
  useEffect(() => {
    if (watching !== subject.id) void window.armada.watchJob(subject.id);
  }, [subject.id, watching]);
  const drones = useMemo(() => droneViewsOf(jobDrones?.state === "read" && jobDrones.jobId === subject.id ? jobDrones.drones : undefined, whole ?? undefined), [jobDrones, subject.id, whole]);
  return { whole, drones };
}

/** The Job's run, the canvas its own page draws. */
function RunCanvas({ subject, props, run }: { subject: Subject; props: CockpitProps; run: Run }) {
  const { holds } = props.state;
  const { whole, drones } = run;
  if (whole === null) return <div className="deck-job__reading" role="status" aria-busy aria-label="Reading the run" />;
  return (
    <JobRun
      whole={whole}
      drones={drones}
      board={props.state.jobs}
      now={props.nows?.[subject.id]}
      workflows={holds.workflows}
      manifest={holds.manifests.find((one) => one.id === whole.job.owner_manifest_id)}
      onOpenJob={props.onOpen}
    />
  );
}

/** The two takes of a Job's run, in its head: the mission across the field it works, and the canvas of its steps. `v` swaps them. */
const JOB_VIEWS = [
  { id: "mission", label: "Mission", hint: "Mission view" },
  { id: "canvas", label: "Canvas", hint: "Canvas view" },
];

/** `mission` draws the Job's mission from the run it is handed: it lives with the field's readings, in the Deck. */
export function JobPane({ subject, props, acts, onPr, call, mission }: { subject: Subject; props: CockpitProps; acts: Acts; onPr: (pr: Pr) => void; call?: Item | undefined; mission: (run: Run) => ReactNode }) {
  const nowing = nowPanelOf(props.nows?.[subject.id], { onOpenJob: props.onOpen, onSaid: () => {} });
  const tone = toneOf(subject);
  const { jobView } = useDeck();
  const run = useRun(subject, props);
  return (
    <section className="deck-pane" aria-label={`Job: ${subject.title}`}>
      <PaneHead glyph={Box} kind="Job" title={subject.title} where={subject.repo} mark={<MarkGlyph mark={{ ...subject.mark, tone }} />}>
        <Tabs items={JOB_VIEWS} value={jobView} onChange={(id) => setJobView(id === "canvas" ? "canvas" : "mission")} />
        <SubjectActs subject={subject} acts={acts} onPr={onPr} />
      </PaneHead>
      {call === undefined || (nowing?.asks?.length ?? 0) > 0 ? null : <CallStrip item={call} props={props} />}
      <div className="deck-job">
        {jobView === "mission" ? (
          <div className="deck-job__mission" role="group" aria-label="Mission">
            {mission(run)}
          </div>
        ) : (
          <div className="deck-job__canvas" role="group" aria-label="Canvas">
            {subject.step === undefined ? null : (
              <div className="deck-job__steps">
                <StepsBar step={subject.step} tone={tone} />
              </div>
            )}
            <div className="deck-job__run">
              <RunCanvas subject={subject} props={props} run={run} />
            </div>
          </div>
        )}
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

export function SlotPane({ view, repo, known }: { view: BayView; repo: string; known: ReadonlySet<string> }) {
  const slot = view.slot;
  const [asked, setAsked] = useState<string>();
  const [scrapping, setScrapping] = useState(false);
  const stranded = view.state === "stranded";
  const rescue = slot.rescue;
  const toContinue = rescue?.verdict === "unfinished" || (rescue?.verdict === undefined && (rescue?.summary !== undefined || (rescue?.items ?? []).length > 0));
  // A holder the ship still holds opens as a link; one that is gone is named by its id.
  const who = (one: BayHolder | undefined) =>
    one === undefined ? undefined : known.has(`${one.kind}:${one.id}`) ? <Holder kind={one.kind} id={one.id} title={one.title ?? one.id} /> : <span className="deck-link__plain">{one.title ?? shortOf(one.id)}</span>;
  const files = slot.stranded?.uncommitted ?? [];
  const commits = slot.stranded?.commits ?? [];
  const mark = <MarkGlyph mark={{ Glyph: view.Glyph, said: view.said ?? view.word, tone: stranded ? "ask" : "calm" }} />;
  return (
    <section className="deck-pane" aria-label={`Bay: slot-${slot.slot}`}>
      <PaneHead glyph={Rows3} kind="Bay" title={`slot-${slot.slot}`} where={repo} mark={mark} />
      <div className="deck-pane__body">
        <Facts
          rows={[
            ["State", view.said ?? view.word],
            ["Holder", who(view.holder)],
            ["Branch", view.branch, true],
            ["Held for", view.heldFor === undefined ? undefined : spanOf(view.heldFor)],
            ["Given back", view.freed === undefined ? undefined : `${spanOf(view.freed)} ago`],
            ["Last holder", stranded ? (who(view.last) ?? "Holder gone") : undefined],
            ["Told", view.told === undefined ? undefined : clockOf(view.told)],
            ["Uncommitted", files.length === 0 ? undefined : files.join("\n"), true],
            ["Unpushed", commits.length === 0 ? undefined : commits.filter((one) => one.home === "only_here").map((one) => `${one.sha.slice(0, 7)} ${one.subject}`).join("\n"), true],
            ["Base", slot.base, true],
            ["Behind", (slot.behind ?? 0) > 0 ? `${slot.behind} commits` : undefined],
            ["Checkout", slot.warm ? "Warm" : "Cold"],
            ["Path", slot.path, true],
          ]}
        />
        {!stranded ? null : (
          <div className="deck-slot__rescue">
            <p className="deck-slot__means">Its holder is gone and the work is still in it. Scrap loses it.</p>
            <SlotFinding slot={slot} />
            {scrapping ? (
              <ScrapConfirm
                slot={slot}
                onScrap={() => {
                  setScrapping(false);
                  setAsked("Scrap");
                }}
                onCancel={() => setScrapping(false)}
              />
            ) : (
              <div className="deck-slot__acts">
                {toContinue ? (
                  <Button variant="secondary" size="sm" disabled={asked !== undefined} onClick={() => setAsked("Pick up")}>
                    Pick up
                  </Button>
                ) : null}
                <Button variant="secondary" size="sm" disabled={asked !== undefined} onClick={() => setAsked("Stash")}>
                  Stash
                </Button>
                <Button variant="destructive" size="sm" disabled={asked !== undefined} onClick={() => setScrapping(true)}>
                  Scrap
                </Button>
                {asked === undefined ? null : <span className="deck-slot__asked">{asked}</span>}
              </div>
            )}
          </div>
        )}
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
