// A plan group, as the Blueprint lists it. Drafted, it names the type of subagent that takes it. Approved, it is its title over a line for each task, the
// one being worked lit with what the turn is doing under it; the subagent type and the paths open with the group. Its approval is a card in the thread.
// Edits Fleet saw are marked on the group. Nothing here blocks.

import { useState } from "react";
import { Bot, Check, ChevronDown, Dot, Link, PencilLine, Ticket } from "lucide-react";
import { Button, LinkedSession, TASK_GLYPH, TaskMark, Tooltip } from "@armada/components";
import { issueLabel, type Session } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { GroupPullRequests, issueOfGroup, openIssuePanel, type Issue } from "./IssuesBlock";
import { GroupEditor, type PlanEditing } from "./PlanEdit";
import { childrenOf, linkedPropsOf } from "./linked";
import type { ViewGroup } from "./plan-marks";
import { wasOpen } from "./place";

/** The issue a group names, state glyph and number or key, a press target of its own beside the group's toggle. */
function GroupIssue({ issue, manifestId }: { issue: Issue; manifestId: string | undefined }) {
  const said = issue.state === "closed" ? "Closed" : "Open";
  return (
    <button type="button" className="sv-group__issue" data-state={issue.state} aria-label={`Issue ${issueLabel(issue.number)} ${issue.title}`} onClick={() => openIssuePanel(issue, manifestId)}>
      <Tooltip label={said}>
        <span className="sv-group__issue-mark" role="img" aria-label={said}>
          <Ticket size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
      <code>{issueLabel(issue.number)}</code>
    </button>
  );
}

/** Where a group stands in the plan: finished, the one being worked, or still to come. */
export type Step = "done" | "current" | "upcoming";

const LEAD_SAID: Record<Step, string> = { done: "Done", current: "Working on this now", upcoming: "Not started" };

/** The mark that leads a group's row in the stepper: a check once done, the breathing mark while it is the one being worked, a dot while it waits. */
export function StepMark({ step }: { step: Step }) {
  const said = LEAD_SAID[step];
  return (
    <Tooltip label={said}>
      {step === "current" ? (
        <span className="armada-session-mark armada-task-mark" data-state="working" data-pulsing role="img" aria-label={said}>
          <TASK_GLYPH.working size={12} strokeWidth={2} aria-hidden />
        </span>
      ) : (
        <span className="sv-group__lead" data-step={step} role="img" aria-label={said}>
          {step === "done" ? <Check size={12} strokeWidth={2} aria-hidden /> : <Dot size={12} strokeWidth={2} aria-hidden />}
        </span>
      )}
    </Tooltip>
  );
}

/** The task a group's live line is about: the first one running. */
const hereOf = (group: ViewGroup): string | undefined => group.tasks.find((task) => task.state === "working")?.id;

/** A closed approved group's tasks: a state glyph and the plan's words for each, the one being worked lit with what the turn is doing under it. */
function Tasks({ group, now }: { group: ViewGroup; now: string | undefined }) {
  const here = hereOf(group);
  return (
    <ul className="sv-group__tasks" aria-label="Tasks">
      {group.tasks.map((task) => (
        <li key={task.id} className="sv-group__task" data-state={task.state} data-here={task.id === here || undefined}>
          {task.id === here ? (
            <Tooltip label="Working on this now">
              <span className="armada-session-mark armada-task-mark" data-state="working" data-pulsing role="img" aria-label="Working on this now">
                <TASK_GLYPH.working size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : (
            <TaskMark state={task.state} />
          )}
          <span className="sv-group__task-text">{task.text}</span>
          {task.id === here && now !== undefined ? <span className="sv-group__task-now">{now}</span> : null}
        </li>
      ))}
    </ul>
  );
}

/**
 * A group is a row that opens: closed it is its title over a line for each task; open it shows the type of subagent, its own paths and its tasks in full.
 * **A group handed to a Session carries that Session's row** under its own; one not handed
 * offers the hand off in its open body.
 */
export function Group({ group, now, step, opened = false, ed, parent, manifestId, onOpenSession }: { group: ViewGroup; /** What the turn is doing in this group, in his words, drawn under the task being worked. */ now?: string; /** Stands open until he presses it: the blueprint of a plan being drafted shows every task. */ opened?: boolean; /** Draws the group as a step of the plan: done and to-come ones as one muted line, the current one open and raised. */ step?: Step; ed: PlanEditing; parent: Session | undefined; manifestId?: string | undefined; onOpenSession: (id: string) => void }) {
  // A stepper's current group stands open until he presses it, and the others shut; a press is kept. Outside a stepper a group stands shut.
  const [pressed, setPressed] = useState<boolean | undefined>(() => (wasOpen(ed.id, group.id) ? true : undefined));
  const open = pressed ?? (opened || step === "current");
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const child = parent === undefined ? undefined : childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const handOff = draft?.handOff;
  // The issue it names, when the Session holds it: the group is that issue's place, with its pull requests under the tasks.
  const named = parent === undefined ? undefined : issueOfGroup(parent, group.issue);
  const live = group.tasks.some((task) => task.state === "working");
  const complete = group.tasks.every((task) => task.state === "done" || task.state === "handed_in");
  const failed = group.tasks.some((task) => task.state === "failed");
  const agent = (
    <Tooltip label="Subagent type">
      <span className="sv-group__agent" role="img" aria-label={`Subagent type ${group.agent}`}>
        <Bot size={12} strokeWidth={2} aria-hidden />
        <code>{group.agent}</code>
      </span>
    </Tooltip>
  );
  return (
    <section className="sv-group" data-group={group.id} data-step={step} data-state={failed ? "failed" : complete ? "done" : live ? "live" : undefined} data-open={open || undefined} data-issue={named === undefined ? undefined : ""} aria-label={group.title}>
      {named === undefined ? null : <GroupIssue issue={named.issue} manifestId={manifestId} />}
      <button type="button" className="sv-group__row" aria-expanded={open} aria-label={group.title} onClick={() => setPressed(!open)}>
        {step === undefined ? null : <StepMark step={step} />}
        <span className="sv-group__title">{group.title}</span>
        <span className="sv-group__head">
          {group.tasks.some((task) => task.seen === true) ? (
            <Tooltip label="Edits seen">
              <span className="sv-group__seen" role="img" aria-label="Edits seen">
                <PencilLine size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : null}
          <ChevronDown className="sv-group__chevron" size={12} strokeWidth={2} aria-hidden />
        </span>
        {step === undefined ? null : agent}
      </button>
      {open || step !== undefined ? null : <Tasks group={group} now={now} />}
      {child === undefined ? null : (
        <div className="sv-group__linked">
          <LinkedSession {...linkedPropsOf(child, onOpenSession)} />
        </div>
      )}
      {open ? (
        <div className="sv-expand">
          <div className="sv-group__body">
            <GroupEditor group={group} ed={ed} head={step === undefined} />
            {child !== undefined || handOff === undefined ? null : (
              <Button variant="ghost" size="sm" onClick={() => void handOff(ed.id, group.id)}>
                <Link size={12} strokeWidth={2} aria-hidden />
                Hand to a Job
              </Button>
            )}
          </div>
        </div>
      ) : null}
      {named === undefined || group.issue === undefined || named.prs.length === 0 ? null : (
        <div className="sv-group__prs">
          <GroupPullRequests prs={named.prs} issue={group.issue} manifestId={manifestId} />
        </div>
      )}
    </section>
  );
}
