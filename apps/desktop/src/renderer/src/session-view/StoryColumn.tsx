// The Job's right column as one story in three zones. **Done** is what the Job has made, in the order it made it (`ledger.ts`), read By step (each finished
// group or workflow step with its outputs beneath it) or By kind (Documents, Pull requests, Issues and Artifacts, newest first). **Now** is what it is doing
// this minute: the brief and the agent before any plan, the group being worked with its live line, subagents, Checks, watches, and whatever waits on him,
// which pulses. **Next** is the plan's groups still to come, the workflow's steps still to come, the outputs the route still owes (a pull request, its
// ci, the merge queue, main; a document; a mod and its Check; findings) and the follow-ups not yet settled. The plan is the spine: a finished group moves to
// Done carrying what it produced, and a pending output moves there when its data arrives.
//
// Two takes of the one story (`story-take.ts`, `?take=`): `spine` draws the zones on one timeline line with a NOW marker, and `stacked` draws them as
// three panels. Keys: ⌥1, ⌥2 and ⌥3 open or fold Done, Now and Next and put the focus on the band; ⌥L swaps By step and By kind; ⌥N the whole plan; ⌥W the
// workflow picker; ↑ ↓ Home End move between the rows of a zone; ← → move the toggle when it has the focus.

import { Fragment, useEffect, useRef, type ReactNode } from "react";
import { ListTree, PencilRuler } from "lucide-react";
import { Button, Tabs, TaskMark, Tooltip, type LedgerEntry } from "@armada/components";
import { useLayout } from "@armada/shell";
import { owesOf, type Session, type SessionWork, type SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { remindLater } from "../cockpit/deck/acts";
import { accountGroupOf, accountNowOf } from "../session-activity";
import { useModsMade } from "../mods-made";
import { useSessionsDraft } from "../sessions-draft";
import { AgentTasks } from "./AgentTasks";
import { artifactsOf } from "./artifacts";
import { Mark } from "./DeliveryRoute";
import { Brief, briefOf, FlightRows } from "./FlightPlan";
import { LaterBlock } from "./LaterBlock";
import { byKind, madeOf, reportOf, type Made } from "./ledger";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import { viewOf } from "./plan-marks";
import { routeOf } from "./route";
import { Flow } from "./StatusPanel";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { AskBeat, DoneGroup, MadeList, RouteBeat, StepBeat, WorkBeat, arrowsIn } from "./StoryParts";
import { SvPanel, type PanelState } from "./SvPanel";
import { TurnAccountBlock, type LiveOpen } from "./TurnAccountBlock";
import { WatchingBlock } from "./WatchingBlock";
import { bandKeyFor, landingOf, useFold, useWhole, WHOLE_KEY } from "./story";
import { storyTake, useLedgerView, type LedgerView } from "./story-take";
import "./progress-takes.css";
import "./story.css";
import "./story-zones.css";

const VIEWS = [
  { id: "step", label: "By step" },
  { id: "kind", label: "By kind" },
];

/** The step of a workflow that holds the plan: the one named for planning, else the first. */
const PLANNING = /plan|draft|design|scope/i;

export function StoryColumn({ session, entries, work, id, manifestId, acts, ed, onOpenSketch, onOpenSession, live }: { session: Session; entries: readonly LedgerEntry[]; work: SessionWork; id: string; manifestId: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void; live: LiveOpen | undefined }) {
  const take = storyTake();
  const draft = useSessionsDraft();
  const layout = useLayout("job.panels").all;
  const [view, setView] = useLedgerView();
  const everything = useWhole(id);
  const doneFold = useFold(session, "done");
  const nowFold = useFold(session, "now");
  const nextFold = useFold(session, "next");
  const mods = useModsMade(id).mods;
  const kind = landingOf(session);

  const plan = work.plan;
  const planned = plan !== undefined && ed !== undefined;
  const drafting = planned && !plan.approved;
  const approved = planned && plan.approved;
  // The plan stands whole in Now while it is drafted, with marks on it, or when he asked to see all of it; its groups are not drawn again in Done and Next.
  const whole = planned && (drafting || everything.whole || ed.marks.length > 0);
  const workingGroup = accountGroupOf(session)?.id;
  const now = accountNowOf(session);
  const groups = planned ? viewOf(plan, ed.marks) : [];
  const steps = stepsOf(groups, approved, workingGroup);
  const by = (step: "done" | "current" | "upcoming") => (whole ? [] : groups.filter((group) => steps.get(group.id) === step));
  const finished = by("done");
  const working = by("current");
  const upcoming = by("upcoming");
  const broke = approved && groups.some((group) => group.tasks.some((task) => task.state === "failed"));

  const flow = work.workflow?.steps ?? acts?.workflows.find((one) => one.id === work.proposal?.workflow)?.steps ?? [];
  const at = work.workflow?.at;
  const workflowDone = at === undefined ? [] : flow.filter((_, i) => i < at);
  const workflowNow = at === undefined ? undefined : flow[at];
  const workflowNext = flow.filter((_, i) => at === undefined || i > at);
  const planning = Math.max(0, workflowDone.findIndex((one) => PLANNING.test(one)));

  const made = madeOf(session, entries, { landing: kind, mods: mods.length, onOpenSketch });
  const doc = kind === "report" ? reportOf(session, entries) !== undefined : artifactsOf(session, entries).some((group) => group.id === "doc");
  const route = routeOf(session, kind, { doc, mods });
  // A step of the route the workflow already names is that step, not a second row.
  const named = (label: string) => flow.some((name) => name.toLowerCase() === label.toLowerCase());
  const ahead = route.filter((one) => one.state === "open" && !named(one.label));
  const underway = route.filter((one) => (one.state === "working" || one.state === "failed") && one.label !== "Check" && !named(one.label));
  const checks = session.rows.filter((one) => one.kind === "check");
  const lastCheck = checks[checks.length - 1];
  const checking = checks.filter((one) => one.kind === "check" && one.state === "running");
  const failedCheck = lastCheck !== undefined && lastCheck.kind === "check" && lastCheck.state === "failed" ? lastCheck : undefined;
  const walks = session.rows.filter((one) => one.kind === "window" && one.ended === undefined);
  const owed = owesOf(session).filter((one) => !(drafting && one.source === "plan"));
  // A question the agent is held on is a call on him as well; it is asked in the thread, and here it is the lamp.
  const asking = session.asked?.questions?.[0]?.question ?? (session.asked === undefined ? undefined : session.asked.command);
  const tasks = planned ? [] : (work.tasks ?? []);
  const atWork = planned ? [] : subagentsAtWork(session);
  const kept = (session.later ?? []).filter((one) => one.confirmed !== true);
  const brief = briefOf(session);
  const working_ = session.turn.state === "working";

  const prs = made.filter((one) => one.shape === "pr");
  const landed = prs.length > 0 && prs.every((one) => one.shape === "pr" && one.pr.state === "merged");
  const asks = owed.length > 0 || walks.length > 0 || asking !== undefined;
  const nowState: PanelState | undefined = asks ? "asks" : broke || failedCheck !== undefined ? "failed" : drafting ? "draft" : working_ || working.length > 0 ? "live" : undefined;
  const doneState: PanelState | undefined = landed ? "done" : undefined;

  // The step each made thing stands under in By step: a finished group takes its own issue and pull requests, the planning step takes the plan.
  const finishedIds = new Set(finished.map((group) => group.id));
  const underStep = (one: Made) => one.key === "plan" && workflowDone.length > 0;
  const underGroup = (one: Made) => one.group !== undefined && finishedIds.has(one.group);
  const loose = made.filter((one) => !underStep(one) && !underGroup(one));
  const planMade = made.find(underStep);
  const planOpen = planMade?.shape === "row" ? planMade.row.open : undefined;
  const sketchOf = plan?.sketch;

  const sketchButton =
    sketchOf === undefined || !drafting ? null : (
      <Tooltip label="Sketch">
        <Button variant="ghost" size="sm" iconOnly aria-label="Sketch" onClick={() => onOpenSketch(sketchOf.id)}>
          <PencilRuler size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    );

  const switcher = (
    <Tabs items={VIEWS} value={view} onChange={(next) => setView(next as LedgerView)} />
  );

  // A zone with nothing in it is its band alone: an empty slot stays empty.
  const briefHere = brief !== undefined && plan !== undefined;
  const hasDone = view === "step" ? briefHere || workflowDone.length > 0 || finished.length > 0 || loose.length > 0 : made.length > 0;
  const done = (
    <SvPanel label="Done" area="done" place="side" state={doneState} acts={switcher} fold={doneFold} keycap={bandKeyFor("done")}>
      {!hasDone ? undefined : (
      <div className="sv-nav" onKeyDown={arrowsIn}>
        {view === "step" ? (
          <ol className="sv-beats" aria-label="By step">
            {!briefHere ? null : (
              <li className="sv-beat sv-beat--brief">
                <Brief text={brief} />
              </li>
            )}
            {workflowDone.map((name, i) => (
              <StepBeat key={name} name={name} state="done" open={i === planning ? planOpen : undefined} />
            ))}
            {finished.map((group) => (
              <DoneGroup key={group.id} group={group} session={session} onOpenSession={onOpenSession} />
            ))}
            {loose.length === 0 ? null : (
              <li className="sv-beat sv-beat--made">
                <MadeList session={session} list={loose} />
              </li>
            )}
          </ol>
        ) : (
          byKind(made).map(({ id: bucket, label, made: list }) => (
            <section key={bucket} className="sv-kind" aria-label={label}>
              <h4 className="sv-kind__label">{label}</h4>
              <MadeList session={session} list={list} />
            </section>
          ))
        )}
      </div>
      )}
    </SvPanel>
  );

  const nowActs = (
    <>
      {sketchButton}
      <AskUpdate subject={`session:${id}`} />
      {!approved ? null : (
        <Tooltip label="Whole plan" shortcut={WHOLE_KEY.cap}>
          <Button variant="ghost" size="sm" iconOnly aria-label="Whole plan" aria-pressed={whole} onClick={everything.toggle}>
            <ListTree size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
    </>
  );

  const nowBody = (
    <div className="sv-nav" onKeyDown={arrowsIn}>
      <ul className="sv-beats" aria-label="Now">
        {asking === undefined ? null : <AskBeat said="Waiting on you" text={asking} />}
        {owed.map((one) => (
          <AskBeat key={one.id} said="Waiting on you" text={one.text} />
        ))}
        {walks.map((one) => (one.kind === "window" ? <AskBeat key={one.id} said="Walk waiting on you" text={one.title} /> : null))}
        <WorkBeat
          mark={<TaskMark state={working_ ? "working" : "open"} />}
          name="Agent"
          line={working.length === 0 ? now : undefined}
        />
        {workflowNow === undefined ? null : <StepBeat name={workflowNow} state="now" />}
        {underway.map((step) => (
          <RouteBeat key={step.key} step={step} />
        ))}
        {checking.map((one) => (one.kind === "check" ? <WorkBeat key={one.id} mark={<Mark state="working" />} name={one.name} /> : null))}
        {failedCheck === undefined ? null : <WorkBeat mark={<Mark state="failed" />} name={failedCheck.name} />}
      </ul>
      {plan !== undefined || brief === undefined ? null : <Brief text={brief} />}
      {tasks.length === 0 ? null : <AgentTasks tasks={tasks} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
      {session.live === undefined || live === undefined ? null : (
        <div className="sv-field__now">
          <TurnAccountBlock session={session} live={live} />
        </div>
      )}
      {working.length === 0 ? null : <FlightRows groups={working} workingGroup={workingGroup} now={now} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />}
      {!planned || !whole ? null : (
        <div className="sv-plan" data-drafting={drafting || undefined}>
          {drafting && plan.summary !== undefined ? <p className="sv-blueprint__summary">{plan.summary}</p> : null}
          <div className="sv-plan__groups">
            {groups.map((group) => {
              const step = steps.get(group.id);
              return <Group key={group.id} group={group} {...(drafting && step !== undefined ? { step, opened: true } : group.id === workingGroup && now !== undefined ? { now } : {})} ed={ed} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />;
            })}
          </div>
          <div className="sv-plan__foot">
            <AddGroup ed={ed} />
            <span className="sv-plan__acts">
              <SendChanges ed={ed} />
              {!drafting || ed.approve === undefined ? null : (
                <Tooltip label={ed.marks.length > 0 ? "Approve with these changes" : "Approve"}>
                  <Button variant="primary" size="sm" onClick={ed.approve}>
                    Approve
                  </Button>
                </Tooltip>
              )}
            </span>
          </div>
        </div>
      )}
      <WatchingBlock key={session.id} session={session} />
    </div>
  );

  const now_ = (
    <SvPanel label="Now" area="now" place="side" state={nowState} acts={nowActs} fold={nowFold} keycap={bandKeyFor("now")}>
      {nowBody}
    </SvPanel>
  );

  const hasNext = upcoming.length > 0 || workflowNext.length > 0 || ahead.length > 0 || kept.length > 0;
  const next = (
    <SvPanel label="Next" area="next" place="side" fold={nextFold} keycap={bandKeyFor("next")}>
      {!hasNext ? undefined : (
      <div className="sv-nav" onKeyDown={arrowsIn}>
        {upcoming.length === 0 ? null : <FlightRows calm groups={upcoming} workingGroup={undefined} now={undefined} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />}
        {workflowNext.length === 0 && ahead.length === 0 ? null : (
          <ol className="sv-beats" aria-label="Ahead">
            {workflowNext.map((name) => (
              <StepBeat key={name} name={name} state="next" />
            ))}
            {ahead.map((step) => (
              <RouteBeat key={step.key} step={step} />
            ))}
          </ol>
        )}
        {kept.length === 0 ? null : <LaterBlock later={kept} issues={[]} manifestId={manifestId} onRemind={(sentence) => void remindLater(draft, id, sentence)} onConfirm={(one) => draft?.confirmLater?.(id, one.id)} onDrop={(one) => draft?.dropLater?.(id, one.id, true)} />}
      </div>
      )}
    </SvPanel>
  );

  const zones: Record<string, ReactNode> = { done, now: now_, next };
  const drawn = layout.map((one) => <Fragment key={one.id}>{zones[one.id]}</Fragment>);
  const head =
    work.railOff !== true && (work.workflow !== undefined || (work.proposal !== undefined && acts !== undefined)) ? (
      <div className="sv-story__head sv-flight__flow">
        <Flow work={work} acts={acts} id={id} manifestId={manifestId} />
      </div>
    ) : null;
  return take === "stacked" ? (
    <>
      {head}
      {drawn}
    </>
  ) : (
    <Spine session={session}>
      {head}
      {drawn}
    </Spine>
  );
}

/** The one timeline: a line down the column with the zones on it. It opens with NOW in view, since the oldest of Done can be long. */
function Spine({ session, children }: { session: Session; children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const column = box.current;
    const now = column?.querySelector<HTMLElement>('[data-area="now"]');
    if (column === null || now === null || now === undefined) return;
    column.scrollTop = Math.max(0, now.offsetTop - column.clientHeight * 0.25);
  }, [session.id]);
  return (
    <div ref={box} className="sv-spine" data-take="spine">
      {children}
    </div>
  );
}
