// A plan group, as the Blueprint lists it. Approved, it is its title over a line for each task, the one being worked lit with what the turn is doing
// under it. Drafted, it is its title over its tasks, editable. Either way the subagent type opens with the group, by its chevron or its pencil, and the
// title of a group that works an issue opens the issue.
// Edits Fleet saw are marked on the group. Nothing here blocks.

import { useState, type ReactNode } from "react";
import { Check, ChevronDown, Dot, Link, PencilLine, Ticket } from "lucide-react";
import { Button, LinkedSession, TASK_GLYPH, TaskMark, Tooltip } from "@armada/components";
import { issueLabel, type Session } from "@armada/screens/src/draft/sessions";

import { useSessions, useSessionsDraft } from "../sessions-draft";
import { GroupPullRequests, issueOfGroup, openIssuePanel, type Issue } from "./IssuesBlock";
import { EditText, GroupEditor, type PlanEditing } from "./PlanEdit";
import { childrenOf, linkedPropsOf } from "./linked";
import { rename, type ViewGroup } from "./plan-marks";
import { wasOpen } from "./place";
import { TaskAge, agedAt, useStale } from "./TaskAge";

/**
 * The issue a group names, on a meta line of its own under the title: the ticket glyph, the key and the issue's title, muted and cut with an ellipsis, all one
 * press that opens the issue. A tracker's key can be long, so it never shares the title's line.
 */
export function GroupIssue({ issue, manifestId }: { issue: Issue; manifestId: string | undefined }) {
  return (
    <Tooltip asChild label={issue.state === "closed" ? `${issue.title}, closed` : issue.title}>
      <button type="button" className="sv-issue" data-state={issue.state} aria-label={`Issue ${issueLabel(issue.number)} ${issue.title}`} onClick={() => openIssuePanel(issue, manifestId)}>
        <span className="sv-issue__mark" aria-hidden="true">
          <Ticket size={12} strokeWidth={2} aria-hidden />
        </span>
        <code className="sv-issue__key">{issueLabel(issue.number)}</code>
        <span className="sv-issue__title">{issue.title}</span>
      </button>
    </Tooltip>
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
export const hereOf = (group: ViewGroup): string | undefined => group.tasks.find((task) => task.state === "working")?.id;

/**
 * One task's line: its state glyph and the plan's words, with what the turn is doing under it where it is the one being worked. A mark in progress that carries
 * the time of its last change (`TaskAge`) reads that age at the right, and dims once it is old; it then claims nothing about what the turn is on, which the
 * detected line at the head of Now says.
 */
export function TaskLine({ task, here, now, issue }: { task: ViewGroup["tasks"][number]; here: boolean; now: string | undefined; /** The issue its group works, on a line of its own under the live line. */ issue?: ReactNode }) {
  const since = agedAt(task);
  const stale = useStale(since);
  const aged = since !== undefined;
  const lit = here && !aged;
  return (
    <li className="sv-group__task" data-state={task.state} data-here={lit || undefined} data-aged={aged || undefined} data-stale={stale || undefined}>
      {lit ? (
        <Tooltip asChild label="Working on this now">
          <span className="armada-session-mark armada-task-mark" data-state="working" data-pulsing role="img" aria-label="Working on this now">
            <TASK_GLYPH.working size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : (
        <TaskMark state={task.state} />
      )}
      <span className="sv-group__task-text">{task.text}</span>
      {since === undefined ? null : <TaskAge since={since} />}
      {lit && now !== undefined ? <span className="sv-group__task-now">{now}</span> : null}
      {issue}
    </li>
  );
}

/** A closed approved group's tasks: a line each, the one being worked lit with what the turn is doing under it. */
function Tasks({ group, now }: { group: ViewGroup; now: string | undefined }) {
  const here = hereOf(group);
  return (
    <ul className="sv-group__tasks" aria-label="Tasks">
      {group.tasks.map((task) => (
        <TaskLine key={task.id} task={task} here={task.id === here} now={now} />
      ))}
    </ul>
  );
}

/**
 * A group is a row that opens: closed it is its title, the issue it works on a line of its own, and a line for each task; open it shows the type of subagent and its tasks in full.
 * A group of a plan being drafted (`opened`) keeps its tasks up to edit; its chevron, or its pencil, brings the subagent type in.
 * **A group handed to a Session carries that Session's row** under its own; one not handed
 * offers the hand off in its open body.
 */
export function Group({ group, now, step, opened = false, ed, parent, manifestId, onOpenSession }: { group: ViewGroup; /** What the turn is doing in this group, in his words, drawn under the task being worked. */ now?: string; /** Stands open until he presses it: the blueprint of a plan being drafted shows every task. */ opened?: boolean; /** Draws the group as a step of the plan: done and to-come ones as one muted line, the current one open and raised. */ step?: Step; ed: PlanEditing; parent: Session | undefined; manifestId?: string | undefined; onOpenSession: (id: string) => void }) {
  // A group stands shut until he presses it, and a press is kept. A drafted group's tasks are up either way: what opens is its head and its paths.
  const [pressed, setPressed] = useState<boolean | undefined>(() => (wasOpen(ed.id, group.id) ? true : undefined));
  const open = pressed ?? false;
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const child = parent === undefined ? undefined : childrenOf(sessions, parent).find((one) => one.group === group.id)?.session;
  const handOff = draft?.handOff;
  // The issue it names, when the Session holds it: the group is that issue's place, with its pull requests under the tasks.
  const named = parent === undefined ? undefined : issueOfGroup(parent, group.issue);
  const live = group.tasks.some((task) => task.state === "working");
  const complete = group.tasks.every((task) => task.state === "done" || task.state === "handed_in");
  const failed = group.tasks.some((task) => task.state === "failed");
  const seen = group.tasks.some((task) => task.seen === true) ? (
    <Tooltip label="Edits seen">
      <span className="sv-group__seen" role="img" aria-label="Edits seen">
        <PencilLine size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  ) : null;
  return (
    <section className="sv-group" data-group={group.id} data-step={step} data-state={failed ? "failed" : complete ? "done" : live ? "live" : undefined} data-open={open || undefined} aria-label={group.title}>
      {opened ? (
        // A drafted group's title is the editable one, so the row holds two presses: the title to rename it and the chevron to open its subagent type and paths.
        <div className="sv-group__row" data-editable="">
          {step === undefined ? null : <StepMark step={step} />}
          <EditText label="Group" className="sv-group__title" value={group.title} was={group.was} onSave={(to) => ed.set(rename(ed.plan, ed.marks, { group: group.id }, to))} />
          <button type="button" className="sv-group__head" aria-expanded={open} aria-label={`Subagent type and paths of ${group.title}`} onClick={() => setPressed(!open)}>
            {seen}
            <ChevronDown className="sv-group__chevron" size={12} strokeWidth={2} aria-hidden />
          </button>
        </div>
      ) : named !== undefined ? (
        // A group that works an issue is that issue: its title opens the issue over the column, and the chevron opens the group.
        <div className="sv-group__row" data-split="">
          {step === undefined ? null : <StepMark step={step} />}
          <Tooltip asChild label="Open the issue">
            <button type="button" className="sv-group__title" onClick={() => openIssuePanel(named.issue, manifestId)}>
              {group.title}
            </button>
          </Tooltip>
          <button type="button" className="sv-group__head" aria-expanded={open} aria-label={`Subagent type of ${group.title}`} onClick={() => setPressed(!open)}>
            {seen}
            <ChevronDown className="sv-group__chevron" size={12} strokeWidth={2} aria-hidden />
          </button>
        </div>
      ) : (
        <button type="button" className="sv-group__row" aria-expanded={open} aria-label={group.title} onClick={() => setPressed(!open)}>
          {step === undefined ? null : <StepMark step={step} />}
          <span className="sv-group__title">{group.title}</span>
          <span className="sv-group__head">
            {seen}
            <ChevronDown className="sv-group__chevron" size={12} strokeWidth={2} aria-hidden />
          </span>
        </button>
      )}
      {named === undefined ? null : (
        <div className="sv-group__meta">
          <GroupIssue issue={named.issue} manifestId={manifestId} />
        </div>
      )}
      {open || opened ? null : <Tasks group={group} now={now} />}
      {child === undefined ? null : (
        <div className="sv-group__linked">
          <LinkedSession {...linkedPropsOf(child, onOpenSession)} />
        </div>
      )}
      {open || opened ? (
        <div className="sv-expand">
          <div className="sv-group__body">
            <GroupEditor group={group} ed={ed} titled={false} head={!opened || open} {...(opened ? { onEdit: () => setPressed(true) } : {})} />
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
