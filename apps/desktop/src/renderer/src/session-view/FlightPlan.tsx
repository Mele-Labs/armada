// The pieces the Job's story is read with (`StoryColumn.tsx`): the brief the Job was dispatched with, a row for each plan group (`FlightRows`) with its state glyph, its title
// and, while folded, one slim track of segments for its tasks on the right (`TaskTrack`), and the head with the workflow's picker and steps (`FlightHead`). A group
// opens to its tasks and the live line, and a press opens or shuts any. The whole plan, with its subagent types and paths, is the whole-plan view in Now.

import { useState } from "react";
import { LinkedSession, Tooltip } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { useSessions } from "../sessions-draft";
import { GroupPullRequests, issueOfGroup } from "./IssuesBlock";
import { childrenOf, linkedPropsOf } from "./linked";
import { GroupIssue, hereOf, StepMark, TaskLine, type Step } from "./PlanGroup";
import { stepsOf } from "./PlanSteps";
import type { ViewGroup } from "./plan-marks";
import { Flow, StepRail } from "./StatusPanel";
import "./flight.css";

/** What the Job was dispatched with: the first thing the person sent, else the words held on its start card. */
export function briefOf(session: Session): string | undefined {
  const sent = session.rows.find((row) => row.kind === "message" && row.from.kind === "you");
  const text = sent !== undefined && sent.kind === "message" ? sent.text : session.start?.message;
  const trimmed = text?.trim();
  return trimmed === undefined || trimmed === "" ? undefined : trimmed;
}

export function Brief({ text }: { text: string }) {
  const [whole, setWhole] = useState(false);
  return (
    <button type="button" className="sv-flight__brief" aria-expanded={whole} aria-label="Brief" data-whole={whole || undefined} onClick={() => setWhole(!whole)}>
      {text}
    </button>
  );
}

/** The brief, then the workflow picker and its steps. Draws nothing the Job does not hold: no brief and no workflow leave it out. */
export function FlightHead({ session, work, id, manifestId, acts }: { session: Session; work: SessionWork; id: string; manifestId: string | undefined; acts: SessionWorkActs | undefined }) {
  const flow = work.workflow;
  const brief = briefOf(session);
  const flows = work.railOff !== true && (flow !== undefined || (work.proposal !== undefined && acts !== undefined));
  if (brief === undefined && !flows) return null;
  return (
    <div className="sv-flight__head">
      {brief === undefined ? null : <Brief text={brief} />}
      {!flows ? null : (
        <div className="sv-flight__flow">
          <Flow work={work} acts={acts} id={id} manifestId={manifestId} />
          {flow === undefined ? null : <StepRail steps={flow.steps} at={flow.at} named />}
        </div>
      )}
    </div>
  );
}

/** One slim track for a folded group: a segment for each task, filled by its state. Where the tasks are listed under the group it is not drawn, since the list says it. */
export function TaskTrack({ tasks }: { tasks: ViewGroup["tasks"] }) {
  if (tasks.length === 0) return null;
  return (
    <Tooltip asChild label="Tasks in this group, done and to do">
      <span className="sv-track" role="img" aria-label="Tasks in this group, done and to do" data-many={tasks.length > 12 || undefined}>
        {tasks.map((task) => (
          <span key={task.id} className="sv-track__seg" data-state={task.state} />
        ))}
      </span>
    </Tooltip>
  );
}

export function FlightGroup({ group, step, open, flip, now, parent, manifestId, onOpenSession }: { group: ViewGroup; step: Step; open: boolean; flip: () => void; now: string | undefined; parent: Session; manifestId: string | undefined; onOpenSession: (id: string) => void }) {
  const sessions = useSessions();
  const child = childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const named = issueOfGroup(parent, group.issue);
  const here = hereOf(group);
  return (
    <li className="sv-flight__group" data-group={group.id} data-step={step} data-open={open || undefined}>
      <button type="button" className="sv-flight__row" aria-expanded={open} aria-label={group.title} onClick={flip}>
        <StepMark step={step} />
        <span className="sv-flight__title">{group.title}</span>
        {open ? null : <TaskTrack tasks={group.tasks} />}
      </button>
      {!open ? null : (
        <div className="sv-flight__open">
          {named === undefined ? null : <GroupIssue issue={named.issue} manifestId={manifestId} />}
          <ul className="sv-group__tasks" aria-label="Tasks">
            {group.tasks.map((task) => (
              <TaskLine key={task.id} task={task} here={task.id === here} now={now} />
            ))}
          </ul>
          {child === undefined ? null : <LinkedSession {...linkedPropsOf(child, onOpenSession)} />}
          {named === undefined || group.issue === undefined || named.prs.length === 0 ? null : <GroupPullRequests prs={named.prs} issue={group.issue} manifestId={manifestId} />}
        </div>
      )}
    </li>
  );
}

/**
 * A row for each group of an approved plan. The group being worked is open, else the first not yet done, so a Job just started shows where it begins;
 * a press opens or shuts any group and holds until the group being worked moves on.
 */
export function FlightRows({ groups, workingGroup, now, session, manifestId, onOpenSession, calm = false, all = false }: { calm?: boolean; /** Whole plan: every group open. */ all?: boolean; groups: readonly ViewGroup[]; workingGroup: string | undefined; now: string | undefined; session: Session; manifestId: string | undefined; onOpenSession: (id: string) => void }) {
  const [pressed, setPressed] = useState<Readonly<Record<string, boolean>>>({});
  const steps = stepsOf(groups, true, workingGroup);
  // `calm` opens none by default: the groups still to come, which are read as titles.
  // Every group being worked stands open, since marks that lag can leave several in progress at once.
  const current = calm ? [] : groups.filter((group) => steps.get(group.id) === "current");
  const lit = new Set(all ? groups.map((group) => group.id) : calm ? [] : current.length > 0 ? current.map((group) => group.id) : groups.filter((group) => steps.get(group.id) !== "done").slice(0, 1).map((group) => group.id));
  return (
    <ul className="sv-flight__groups" aria-label="Groups">
      {groups.map((group) => {
        const open = pressed[group.id] ?? lit.has(group.id);
        return <FlightGroup key={group.id} group={group} step={steps.get(group.id) ?? "upcoming"} open={open} flip={() => setPressed({ ...pressed, [group.id]: !open })} now={group.id === workingGroup ? now : undefined} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />;
      })}
    </ul>
  );
}
