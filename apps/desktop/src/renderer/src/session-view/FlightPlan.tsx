// The Blueprint as a flight plan. At the top, the brief the Job was dispatched with and the workflow's steps as one line of glyphs with their names, the
// picker beside them (`FlightHead`). Under it, once a plan is approved, a row for each group (`FlightRows`): its state glyph, its title, and a tick for
// each task on the right, filled where the task is done and hollow where it waits. Only the group being worked opens to its tasks and the live line; a
// press opens another. The whole plan, with its subagent types and paths, is the whole-blueprint view (`BlueprintPanel.tsx`).

import { useState } from "react";
import { Circle, Link } from "lucide-react";
import { Button, LinkedSession, Tooltip, taskMarkSaid, type TaskMarkState } from "@armada/components";
import type { Session, SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "../sessions-draft";
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

function Brief({ text }: { text: string }) {
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

/** A tick for a task: filled once done, hollow while it waits, in the hue of the state. The task's words are its tooltip. */
function Tick({ task }: { task: ViewGroup["tasks"][number] }) {
  const state = task.state as TaskMarkState;
  const said = `${task.text}, ${taskMarkSaid(state).toLowerCase()}`;
  return (
    <Tooltip asChild label={task.text}>
      <span className="sv-tick" data-state={state} role="img" aria-label={said}>
        <Circle size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function FlightGroup({ group, step, open, flip, now, parent, manifestId, onOpenSession }: { group: ViewGroup; step: Step; open: boolean; flip: () => void; now: string | undefined; parent: Session; manifestId: string | undefined; onOpenSession: (id: string) => void }) {
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const child = childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const named = issueOfGroup(parent, group.issue);
  const here = hereOf(group);
  const handOff = draft?.handOff;
  return (
    <li className="sv-flight__group" data-group={group.id} data-step={step} data-open={open || undefined}>
      <button type="button" className="sv-flight__row" aria-expanded={open} aria-label={group.title} onClick={flip}>
        <StepMark step={step} />
        <span className="sv-flight__title">{group.title}</span>
        <span className="sv-ticks" aria-hidden={group.tasks.length === 0 || undefined}>
          {group.tasks.map((task) => (
            <Tick key={task.id} task={task} />
          ))}
        </span>
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
          {child !== undefined || handOff === undefined ? null : (
            <Button variant="ghost" size="sm" onClick={() => void handOff(parent.id, group.id)}>
              <Link size={12} strokeWidth={2} aria-hidden />
              Hand to a Job
            </Button>
          )}
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
export function FlightRows({ groups, workingGroup, now, session, manifestId, onOpenSession }: { groups: readonly ViewGroup[]; workingGroup: string | undefined; now: string | undefined; session: Session; manifestId: string | undefined; onOpenSession: (id: string) => void }) {
  const [pressed, setPressed] = useState<Readonly<Record<string, boolean>>>({});
  const steps = stepsOf(groups, true, workingGroup);
  const lit = groups.find((group) => steps.get(group.id) === "current") ?? groups.find((group) => steps.get(group.id) !== "done");
  return (
    <ul className="sv-flight__groups" aria-label="Groups">
      {groups.map((group) => {
        const open = pressed[group.id] ?? group.id === lit?.id;
        return <FlightGroup key={group.id} group={group} step={steps.get(group.id) ?? "upcoming"} open={open} flip={() => setPressed({ ...pressed, [group.id]: !open })} now={group.id === workingGroup ? now : undefined} parent={session} manifestId={manifestId} onOpenSession={onOpenSession} />;
      })}
    </ul>
  );
}
