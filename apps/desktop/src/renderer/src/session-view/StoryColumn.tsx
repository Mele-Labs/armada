// The Job's right column as one story in three zones. **Completed** is what the Job has made, in the order it made it (`ledger.ts`), read By step (each finished
// group or workflow step with its outputs beneath it) or By kind (Documents, Pull requests, Issues and Artifacts, newest first). **Now** is what it is doing
// this minute: the agent, the group being worked with its live line, subagents, Checks, watches, and whatever waits on him,
// which pulses. **Next** is the plan's groups still to come, the workflow's steps still to come, the outputs the route still owes (a pull request, its
// ci, the merge queue, main; a document; a mod and its Check; findings) and the follow-ups not yet settled. The plan is the spine: a finished group moves to
// Completed carrying what it produced, and a pending output moves there when its data arrives.
//
// With a workflow whose step the Job is at, the zones are the workflow's steps instead: one for each, a check on the line for a finished step (folded until pressed, holding what
// was made during it), the current step open with what Now holds, and the steps to come empty until reached, the last of them holding what Next holds about landing. ⌥1, ⌥2 and ⌥3
// then fold the finished steps, the current step and the steps to come. A Job with no workflow, or with workflows off, keeps Completed, Now and Next.
//
// The Brief is the first beat on the spine, above the zones, and opens in place to the whole text and how the Job started. The zones are drawn on one timeline line (the spine) with a NOW marker. Keys: ⌥1, ⌥2 and ⌥3 open or fold Completed, Now and Next and put the focus on the band; ⌥L swaps By step and By kind, two icon toggles in Completed's strip; ⌥N the whole plan; ⌥W the
// workflow name at the head, which opens the picker; ↑ ↓ Home End move between the rows of a zone; ← → move the toggle when it has the focus.

import { Fragment, useEffect, useRef, useState, type ReactNode } from "react";
import { ListTree, PencilRuler, SkipForward } from "lucide-react";
import { Button, TaskMark, Tooltip, waitingSaid, type LedgerEntry } from "@armada/components";
import { useLayout } from "@armada/shell";
import { owesOf, type Session, type SessionWork, type SessionWorkActs, type WorkGroup } from "@armada/screens/src/draft/sessions";

import { AskUpdate } from "../cockpit/deck/AskUpdate";
import { remindLater } from "../cockpit/deck/acts";
import { accountGroupOf, accountNowOf } from "../session-activity";
import { useModsMade } from "../mods-made";
import { useSessionsDraft } from "../sessions-draft";
import { openInPanel } from "../shared-panel/store";
import { AgentTasks } from "./AgentTasks";
import { detectedOf } from "./detected";
import { artifactsOf } from "./artifacts";
import { Mark } from "./DeliveryRoute";
import { BriefZone } from "./BriefZone";
import { briefOf, FlightRows } from "./FlightPlan";
import { LaterBlock } from "./LaterBlock";
import { OpenGroupMenu, useOpenGroupShown } from "./OpenGroup";
import { WALK_STEP, byKind, madeOf, reportOf, stepOf, type Made } from "./ledger";
import { AddGroup, SendChanges, type PlanEditing } from "./PlanEdit";
import { Group } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import { MARK, stateOf } from "./StatusPanel";
import { viewOf } from "./plan-marks";
import { routeOf } from "./route";
import { SubagentsAtWork } from "./SubagentsAtWork";
import { subagentsAtWork } from "./subagents-at-work";
import { freshestOf } from "./TaskAge";
import { AskBeat, DoneGroup, LedgerToggle, MadeList, RouteBeat, StepBeat, WaitBeat, WalkBeat, WalkToCome, WorkBeat, arrowsIn } from "./StoryParts";
import { SkipStep, skipMenu } from "./StepSkip";
import { SvPanel, type PanelState } from "./SvPanel";
import { TurnAccountBlock, type LiveOpen } from "./TurnAccountBlock";
import { WatchingBlock } from "./WatchingBlock";
import { WorkflowHead } from "./WorkflowHead";
import { bandKeyFor, landingOf, useFold, useWhole, WHOLE_KEY } from "./story";
import { useLedgerView } from "./ledger-view";
import "./progress-takes.css";
import "./story.css";
import "./story-zones.css";

/** The rule a waiting item is the ask of, and its card in the thread: an item `draft:<card>` that a rule on the Job's lanes left. Absent where no rule did. */
function ruleOf(session: Session, itemId: string): { rule: string; row: string } | undefined {
  const row = itemId.startsWith("draft:") ? itemId.slice("draft:".length) : undefined;
  const fired = row === undefined ? undefined : session.lanes?.job.fired.find((one) => one.needs === row);
  return row === undefined || fired === undefined ? undefined : { rule: fired.rule, row };
}

export function StoryColumn({ session, entries, work, id, manifestId, acts, ed, onOpenSketch, onOpenSession, live }: { session: Session; entries: readonly LedgerEntry[]; work: SessionWork; id: string; manifestId: string | undefined; acts: SessionWorkActs | undefined; ed: PlanEditing | undefined; onOpenSketch: (sketch: string) => void; onOpenSession: (id: string) => void; live: LiveOpen | undefined }) {
  const draft = useSessionsDraft();
  const layout = useLayout("job.panels").all;
  const [view, setView] = useLedgerView();
  const everything = useWhole(id);
  const flow = work.workflow?.steps ?? acts?.workflows.find((one) => one.id === work.proposal?.workflow)?.steps ?? [];
  const at = work.workflow?.at;
  // With a workflow whose step the Job is at, the zones are its steps (finished, current, to come); without one, or with workflows off, they are Completed, Now and Next.
  const stepped = work.railOff !== true && flow.length > 0 && at !== undefined;
  const openShown = useOpenGroupShown();
  const [openOpen, setOpenOpen] = useState(true);
  const doneFold = useFold(session, "done", stepped);
  const nowFold = useFold(session, "now");
  const nextFold = useFold(session, "next");
  // A finished step opened or shut by itself; ⌥1 sets them all and clears these.
  const [own, setOwn] = useState<Record<number, boolean>>({});
  useEffect(() => setOwn({}), [doneFold.open]);
  const mods = useModsMade(id).mods;
  const kind = landingOf(session);

  const plan = work.plan;
  const planned = plan !== undefined && ed !== undefined;
  const drafting = planned && !plan.approved;
  const approved = planned && plan.approved;
  // The plan stands whole in Now, to edit, while it is drafted or has marks on it; its groups are not drawn again in Completed and Next. **Whole plan** on
  // an approved plan leaves each group in its own zone and opens every one, so finished work never reads as Now.
  const editing = planned && (drafting || ed.marks.length > 0);
  const whole = planned && (editing || everything.whole);
  const all = !editing && everything.whole;
  const workingGroup = accountGroupOf(session)?.id;
  // What Fleet sees the Job doing leads Now. Without calls to read it from, the account's line stands under the task being worked as before.
  const detected = detectedOf(session);
  const now = detected === undefined ? accountNowOf(session) : undefined;
  const groups = planned ? viewOf(plan, ed.marks) : [];
  const steps = stepsOf(groups, approved, workingGroup);
  const by = (step: "done" | "current" | "upcoming") => (editing ? [] : groups.filter((group) => steps.get(group.id) === step));
  const finished = by("done");
  // The group with the freshest mark in progress leads; groups with no age keep the plan's order.
  const working = by("current").sort((a, b) => (freshestOf(a) === freshestOf(b) ? 0 : freshestOf(b) > freshestOf(a) ? 1 : -1));
  const upcoming = by("upcoming");
  const broke = approved && groups.some((group) => group.tasks.some((task) => task.state === "failed"));

  const workflowDone = at === undefined ? [] : flow.filter((_, i) => i < at);
  const workflowNow = at === undefined ? undefined : flow[at];
  const workflowNext = flow.filter((_, i) => at === undefined || i > at);

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
  // A walk shown to him is drawn from its page's row, which opens it, and not a second time from the item Fleet holds for it.
  const walkUrls = new Set(walks.flatMap((one) => (one.kind === "window" ? [one.url] : [])));
  const owed = owesOf(session).filter((one) => !(drafting && one.source === "plan") && !(one.source === "walk" && one.act?.kind === "walk" && walkUrls.has(one.act.target)));
  // A question the agent is held on is a call on him as well; it is asked in the thread, and here it is the lamp.
  const asking = session.asked?.questions?.[0]?.question ?? (session.asked === undefined ? undefined : session.asked.command);
  const tasks = planned ? [] : (work.tasks ?? []);
  const atWork = planned ? [] : subagentsAtWork(session);
  const kept = (session.later ?? []).filter((one) => one.confirmed !== true);
  const brief = briefOf(session);
  const working_ = session.turn.state === "working";
  // A step that ends on a state of a ticket, a pull request or a deploy waits for it: Fleet's watch wakes the Job on the change.
  const lanes = session.lanes?.job;
  const waiting = lanes === undefined ? undefined : waitingSaid(lanes, lanes.copy.lifecycles);

  const prs = made.filter((one) => one.shape === "pr");
  const landed = prs.length > 0 && prs.every((one) => one.shape === "pr" && one.pr.state === "merged");
  const asks = owed.length > 0 || walks.length > 0 || asking !== undefined;
  const nowState: PanelState | undefined = asks ? "asks" : broke || failedCheck !== undefined ? "failed" : drafting ? "draft" : working_ || working.length > 0 ? "live" : undefined;
  const doneState: PanelState | undefined = landed ? "done" : undefined;

  // The step each made thing stands under in By step: a finished group takes its own issue and pull requests, a finished workflow step takes what it made (`stepOf`).
  const finishedIds = new Set(finished.map((group) => group.id));
  const underGroup = (one: Made) => one.group !== undefined && finishedIds.has(one.group);
  const underStep = (one: Made) => (underGroup(one) ? undefined : stepOf(one, workflowDone));
  const loose = made.filter((one) => !underGroup(one) && underStep(one) === undefined);
  const sketchOf = plan?.sketch;

  const sketchButton =
    sketchOf === undefined || !drafting ? null : (
      <Tooltip label="Sketch">
        <Button variant="ghost" size="sm" iconOnly aria-label="Sketch" onClick={() => onOpenSketch(sketchOf.id)}>
          <PencilRuler size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    );

  const switcher = <LedgerToggle view={view} onChange={setView} />;

  // A zone with nothing in it is its band alone: an empty slot stays empty.
  const hasDone = view === "step" ? workflowDone.length > 0 || finished.length > 0 || loose.length > 0 : made.length > 0;
  const done = (
    <SvPanel label="Completed" area="done" place="side" state={doneState} acts={switcher} fold={doneFold} keycap={bandKeyFor("done")}>
      {!hasDone ? undefined : (
      <div className="sv-nav" onKeyDown={arrowsIn}>
        {view === "step" ? (
          <ol className="sv-beats" aria-label="By step">
            {workflowDone.map((name) => {
              const list = made.filter((one) => underStep(one) === name);
              return <StepBeat key={name} name={name} state="done" {...(list.length === 0 ? {} : { under: <MadeList session={session} list={list} newest="last" /> })} />;
            })}
            {finished.map((group) => (
              <DoneGroup key={group.id} group={group} open={all} session={session} onOpenSession={onOpenSession} />
            ))}
            {loose.length === 0 ? null : (
              <li className="sv-beat sv-beat--made">
                <MadeList session={session} list={loose} newest="last" />
              </li>
            )}
          </ol>
        ) : (
          <>
            {byKind(made).map(({ id: bucket, label, made: list }) => (
              <section key={bucket} className="sv-kind" aria-label={label}>
                <h4 className="sv-kind__label">{label}</h4>
                <MadeList session={session} list={list} newest="first" />
              </section>
            ))}
          </>
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

  // The workflow heads the story, at the top of the spine.
  const head =
    work.railOff !== true && (work.workflow !== undefined || (work.proposal !== undefined && acts !== undefined)) ? <WorkflowHead work={work} acts={acts} id={id} manifestId={manifestId} /> : null;

  const nowBody = (
    <div className="sv-nav" onKeyDown={arrowsIn}>
      <ul className="sv-beats" aria-label="Now">
        {asking === undefined ? null : <AskBeat said="Waiting on you" text={asking} />}
        {owed.map((one) => {
          const rule = ruleOf(session, one.id);
          return <AskBeat key={one.id} said="Waiting on you" text={one.text} {...(rule === undefined ? {} : { onOpen: () => openInPanel({ kind: "rule", sessionId: session.id, ...rule }) })} />;
        })}
        {walks.map((one) => (one.kind === "window" ? <WalkBeat key={one.id} title={one.title} takes={one.takes} onOpen={() => draft?.openWindow?.(id, one.url)} /> : null))}
        <WorkBeat
          mark={<TaskMark state={working_ ? "working" : "open"} />}
          name={detected ?? "Agent"}
          working={detected !== undefined}
          line={working.length === 0 ? now : undefined}
        />
        {workflowNow === undefined || stepped ? null : <StepBeat name={workflowNow} state="now" />}
        {waiting === undefined ? null : <WaitBeat text={waiting} />}
        {underway.map((step) => (
          <RouteBeat key={step.key} step={step} />
        ))}
        {checking.map((one) => (one.kind === "check" ? <WorkBeat key={one.id} mark={<Mark state="working" />} name={one.name} system /> : null))}
        {failedCheck === undefined ? null : <WorkBeat mark={<Mark state="failed" />} name={failedCheck.name} system />}
      </ul>
      {tasks.length === 0 ? null : <AgentTasks tasks={tasks} />}
      {atWork.length === 0 ? null : <SubagentsAtWork rows={atWork} />}
      {session.live === undefined || live === undefined ? null : (
        <div className="sv-field__now">
          <TurnAccountBlock session={session} live={live} />
        </div>
      )}
      {working.length === 0 ? null : <FlightRows all={all} groups={working} workingGroup={workingGroup} now={now} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />}
      {!editing ? null : (
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
        {upcoming.length === 0 ? null : <FlightRows calm all={all} groups={upcoming} workingGroup={undefined} now={undefined} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />}
        {workflowNext.length === 0 && ahead.length === 0 ? null : (
          <ol className="sv-beats" aria-label="Ahead">
            {workflowNext.map((name) => (
              <StepBeat key={name} name={name} state="next" {...(WALK_STEP.test(name) && walks.length === 0 ? { under: <WalkToCome /> } : {})} />
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

  // ---- Zones as the workflow's steps ----
  // A finished thing stands under the step whose visit covers its time: the latest visit that began at or before it. A step the Job went back to is one zone with both
  // visits' things stacked in time order. A thing with no time, a Job with no visits, or a time after the current step began, stands under the last finished step.
  const visits = work.workflow?.visits;
  const lastDone = (at ?? 0) - 1;
  const stepOfTime = (when: string | undefined): number => {
    if (when === undefined || visits === undefined) return lastDone;
    const time = Date.parse(when);
    const hit = visits.reduce((found, one) => (Date.parse(one.since) <= time ? one.step : found), -1);
    return hit < 0 ? 0 : Math.min(hit, lastDone);
  };
  const groupTime = (group: WorkGroup) => group.tasks.reduce<string | undefined>((latest, one) => (one.changedAt !== undefined && (latest === undefined || one.changedAt > latest) ? one.changedAt : latest), undefined);
  const madeTime = (one: Made): string | undefined => {
    if (one.shape === "pr") return session.trail?.opened?.[one.pr.number];
    if (one.shape === "row" && one.key.startsWith("walk:")) {
      const shown = session.rows.find((row) => row.kind === "window" && `walk:${row.id}` === one.key);
      return shown?.kind === "window" ? shown.ended : undefined;
    }
    return undefined;
  };
  // A pull request not yet merged and an issue not yet closed are work in hand, so they never stand under a finished step.
  const stillOpen = (one: Made) => (one.shape === "pr" && one.pr.state !== "merged") || (one.shape === "issue" && one.issue.state === "open");
  const openMade = made.filter(stillOpen);
  const keptBlock = kept.length === 0 ? null : <LaterBlock later={kept} issues={[]} manifestId={manifestId} onRemind={(sentence) => void remindLater(draft, id, sentence)} onConfirm={(one) => draft?.confirmLater?.(id, one.id)} onDrop={(one) => draft?.dropLater?.(id, one.id, true)} />;
  type Done = { time: string | undefined; group: (typeof finished)[number] } | { time: string | undefined; made: Made };
  const finishedThings: Done[] = [
    ...finished.map((group): Done => ({ time: groupTime(group), group })),
    ...made.filter((one) => !underGroup(one) && !stillOpen(one)).map((one): Done => ({ time: madeTime(one), made: one })),
  ];
  const doneIn = (step: number): Done[] =>
    finishedThings.filter((one) => stepOfTime(one.time) === step).sort((a, b) => (a.time === undefined || b.time === undefined ? 0 : a.time < b.time ? -1 : a.time > b.time ? 1 : 0));
  const underDone = (list: Done[]) => {
    const rows: ReactNode[] = [];
    let run: Made[] = [];
    const flush = () => {
      if (run.length > 0) rows.push(<li key={`made-${run[0]!.key}`} className="sv-beat sv-beat--made"><MadeList session={session} list={run} newest="last" /></li>);
      run = [];
    };
    for (const one of list) {
      if ("made" in one) run.push(one.made);
      else {
        flush();
        rows.push(<DoneGroup key={one.group.id} group={one.group} open={all} session={session} onOpenSession={onOpenSession} />);
      }
    }
    flush();
    return <div className="sv-nav" onKeyDown={arrowsIn}><ol className="sv-beats" aria-label="Done">{rows}</ol></div>;
  };

  // What Next held about landing: the route's outputs still to be made and the follow-ups not yet settled. It stands under the last step.
  const landing =
    ahead.length === 0 && kept.length === 0 ? undefined : (
      <div className="sv-nav" onKeyDown={arrowsIn}>
        {ahead.length === 0 ? null : (
          <ol className="sv-beats" aria-label="Ahead">
            {ahead.map((step) => (
              <RouteBeat key={step.key} step={step} />
            ))}
          </ol>
        )}
        {kept.length === 0 ? null : <LaterBlock later={kept} issues={[]} manifestId={manifestId} onRemind={(sentence) => void remindLater(draft, id, sentence)} onConfirm={(one) => draft?.confirmLater?.(id, one.id)} onDrop={(one) => draft?.dropLater?.(id, one.id, true)} />}
      </div>
    );

  // A step the person skipped stays on the spine, muted, with its Undo. Skip is offered on the current step and the ones to come, never on the last,
  // which delivers, nor on one finished.
  const skippedSteps = new Set(work.workflow?.skipped ?? []);
  const skipOf = (i: number, skipped: boolean) => (acts?.skip === undefined ? undefined : () => acts.skip?.(id, i, !skipped));
  const skippedMark = (
    <Tooltip asChild label="Skipped">
      <span className="sv-panel__mark sv-panel__mark--skipped" role="img" aria-label="Skipped">
        <SkipForward size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );

  const stepZones = !stepped
    ? []
    : flow.map((name, i) => {
        const state = stateOf(i, at);
        const last = i === flow.length - 1;
        const isSkipped = skippedSteps.has(i) && !last;
        const press = last ? undefined : skipOf(i, isSkipped);
        // Fleet moves the current step past a skipped one; were it ever at one, it is drawn as current, with its Undo.
        if (isSkipped && state !== "now") {
          return (
            <SvPanel key={`${i}-${name}`} label={name} area={state === "done" ? "step-done" : "step-next"} place="side" marker={skippedMark} skipped acts={press === undefined ? undefined : <SkipStep skipped onPress={press} />} menu={press === undefined ? undefined : skipMenu(true, press)} />
          );
        }
        const skipping = press === undefined || state === "done" ? undefined : { acts: <SkipStep skipped={isSkipped} onPress={press} />, menu: skipMenu(isSkipped, press) };
        if (state === "done") {
          const list = doneIn(i);
          const open = own[i] ?? doneFold.open;
          const { Glyph, said } = MARK.done;
          const marker = (
            <Tooltip asChild label={said}>
              <span className="sv-panel__mark" role="img" aria-label={said}>
                <Glyph size={16} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          );
          return (
            <SvPanel key={`${i}-${name}`} label={name} area="step-done" place="side" marker={marker} fold={{ open, toggle: () => setOwn((was) => ({ ...was, [i]: !open })), current: false }} keycap={bandKeyFor("done")}>
              {list.length === 0 ? undefined : underDone(list)}
            </SvPanel>
          );
        }
        if (state === "now") {
          // Before the first step is reached nothing is finished under another step, so what is finished stands here.
          const early = lastDone < 0 ? finishedThings : [];
          return (
            <SvPanel key={`${i}-${name}`} label={name} area="step-now" place="side" state={nowState} acts={<>{nowActs}{skipping?.acts}</>} menu={skipping?.menu} skipped={isSkipped} fold={nowFold} keycap={bandKeyFor("now")}>
              <>
                {early.length === 0 ? null : underDone(early)}
                {nowBody}
                {upcoming.length === 0 ? null : (
                  <div className="sv-nav" onKeyDown={arrowsIn}>
                    <FlightRows calm all={all} groups={upcoming} workingGroup={undefined} now={undefined} session={session} manifestId={manifestId} onOpenSession={onOpenSession} />
                  </div>
                )}
                {openMade.length === 0 ? null : (
                  <div className="sv-nav" onKeyDown={arrowsIn}>
                    <ol className="sv-beats" aria-label="Open">
                      <li className="sv-beat sv-beat--made">
                        <MadeList session={session} list={openMade} newest="last" />
                      </li>
                    </ol>
                  </div>
                )}
                {last ? landing : keptBlock}
              </>
            </SvPanel>
          );
        }
        return (
          <SvPanel key={`${i}-${name}`} label={name} area="step-next" place="side" acts={skipping?.acts} menu={skipping?.menu} fold={nextFold} keycap={bandKeyFor("next")}>
            {!last && !(WALK_STEP.test(name) && walks.length === 0) ? undefined : (
              <>
                {WALK_STEP.test(name) && walks.length === 0 ? <WalkToCome /> : null}
                {last ? landing : null}
              </>
            )}
          </SvPanel>
        );
      });
  const pinned =
    !stepped || !openShown || (openMade.length === 0 && kept.length === 0) ? null : (
      <OpenGroupMenu>
        <SvPanel label="Open" area="step-next" place="side" fold={{ open: openOpen, toggle: () => setOpenOpen((was) => !was), current: false }} keycap={bandKeyFor("next")}>
          <div className="sv-nav" onKeyDown={arrowsIn}>
            <MadeList session={session} list={openMade} newest="last" />
            {keptBlock}
          </div>
        </SvPanel>
      </OpenGroupMenu>
    );
  return (
    <Spine session={session}>
      {head === null ? null : <div className="sv-story__head">{head}</div>}
      {brief === undefined ? null : <BriefZone key={session.id} session={session} work={work} text={brief} onOpenSession={onOpenSession} />}
      {stepped ? stepZones : drawn}
      {pinned}
    </Spine>
  );
}

/** Puts a zone's header at the top of the spine, the sticky one it stands in for, so the zone reads from its first row. */
export function placeZone(zone: HTMLElement, behavior: ScrollBehavior = "auto"): boolean {
  const column = zone.closest<HTMLElement>(".sv-spine");
  if (column === null) return false;
  column.scrollTo({ top: Math.max(0, zone.offsetTop - parseFloat(getComputedStyle(column).paddingTop)), behavior });
  return true;
}

/** The one timeline: a line down the column with the zones on it. The column is the only thing that scrolls, and each zone's header holds at its top while the zone passes under it. It opens with Now's header near the top: Completed is above, to scroll back to, and Next below. */
function Spine({ session, children }: { session: Session; children: ReactNode }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const now = box.current?.querySelector<HTMLElement>('[data-area="now"], [data-area="step-now"]');
    if (now !== null && now !== undefined) placeZone(now);
  }, [session.id]);
  return (
    <div ref={box} className="sv-spine">
      {children}
    </div>
  );
}
