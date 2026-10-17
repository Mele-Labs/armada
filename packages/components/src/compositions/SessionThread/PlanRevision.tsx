import { useId, useState } from "react";
import { Bot, ChevronDown, ChevronRight, Pencil, Plus, Trash2 } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { TaskMark } from "../TaskMark/TaskMark";
import type { TaskMarkState } from "../TaskMark/TaskMark";

/** What the agent changed on a group or task when it recorded the plan again. */
export type RevisionSince = { kind: "added" } | { kind: "renamed"; from: string };

export type RevisionTask = { id: string; text: string; state: TaskMarkState; since?: RevisionSince };

export type RevisionGroup = {
  id: string;
  agent: string;
  title: string;
  since?: RevisionSince;
  tasks: readonly RevisionTask[];
  /** The text of tasks the agent took out of this group. */
  gone: readonly string[];
};

/**
 * A plan that is a later version of one already worked, drawn in its order with what changed marked where it stands: a group nothing
 * touched is closed to one line, and a group with a change is open, its added, renamed and removed tasks marked.
 */
export type PlanRevision = { groups: readonly RevisionGroup[] };

const changed = (group: RevisionGroup): boolean => group.since !== undefined || group.gone.length > 0 || group.tasks.some((task) => task.since !== undefined);

/** Which unchanged groups are open. The key opens them all, or closes them all once every one is open. */
export function useUnchanged(revision: PlanRevision | undefined) {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const ids = revision === undefined ? [] : revision.groups.filter((group) => !changed(group)).map((group) => group.id);
  const every = ids.length > 0 && ids.every((id) => open.has(id));
  return {
    has: (id: string) => open.has(id),
    toggle: (id: string) =>
      setOpen((was) => {
        const next = new Set(was);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    toggleAll: () => setOpen(every ? new Set() : new Set(ids)),
  };
}

export type Unchanged = ReturnType<typeof useUnchanged>;

const CHANGE: Record<"added" | "renamed" | "removed", { Glyph: LucideIcon; said: string }> = {
  added: { Glyph: Plus, said: "Added" },
  renamed: { Glyph: Pencil, said: "Renamed" },
  removed: { Glyph: Trash2, said: "Removed" },
};

function ChangeMark({ kind }: { kind: keyof typeof CHANGE }) {
  const { Glyph, said } = CHANGE[kind];
  return (
    <Tooltip asChild label={said}>
      <span className="armada-plan-revision__change" data-kind={kind} role="img" aria-label={said}>
        <Glyph size={12} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

function AgentChip({ agent }: { agent: string }) {
  return (
    <Tooltip label="Subagent type">
      <span className="armada-session-thread__agent" role="img" aria-label={`Subagent type ${agent}`}>
        <Bot size={12} strokeWidth={2} aria-hidden />
        <code>{agent}</code>
      </span>
    </Tooltip>
  );
}

/** A group's title, wearing the mark when the agent added or renamed the group itself. */
function GroupTitle({ group }: { group: RevisionGroup }) {
  const since = group.since;
  return (
    <span className="armada-plan-revision__title" data-kind={since?.kind}>
      {since === undefined ? null : <ChangeMark kind={since.kind} />}
      <span className="armada-plan-revision__text">
        {since?.kind === "renamed" ? <s>{since.from}</s> : null}
        <span>{group.title}</span>
      </span>
    </span>
  );
}

/** One task: the change it carries, its state, its text. */
function TaskLine({ task }: { task: RevisionTask }) {
  const since = task.since;
  return (
    <li className="armada-plan-revision__task" data-kind={since?.kind}>
      {since === undefined ? <span aria-hidden /> : <ChangeMark kind={since.kind} />}
      <TaskMark state={task.state} />
      <span className="armada-plan-revision__text">
        {since?.kind === "renamed" ? <s>{since.from}</s> : null}
        <span>{task.text}</span>
      </span>
    </li>
  );
}

function GoneLine({ text }: { text: string }) {
  return (
    <li className="armada-plan-revision__task" data-kind="removed">
      <ChangeMark kind="removed" />
      <span aria-hidden />
      <span className="armada-plan-revision__text">
        <s>{text}</s>
      </span>
    </li>
  );
}

/** One unchanged group: a line that opens to its tasks. */
function Quiet({ group, unchanged }: { group: RevisionGroup; unchanged: Unchanged }) {
  const list = useId();
  const open = unchanged.has(group.id);
  return (
    <li className="armada-plan-revision__group" data-quiet>
      <Tooltip asChild label={open ? "Hide tasks" : "Show tasks"}>
        <button type="button" className="armada-plan-revision__fold armada-plan-revision__head" aria-label={group.title} aria-expanded={open} {...(open ? { "aria-controls": list } : {})} onClick={() => unchanged.toggle(group.id)}>
          {open ? <ChevronDown size={12} strokeWidth={2} aria-hidden /> : <ChevronRight size={12} strokeWidth={2} aria-hidden />}
          <span className="armada-plan-revision__title">
            <span className="armada-plan-revision__text">
              <span>{group.title}</span>
            </span>
          </span>
          <span className="armada-session-thread__marks">
            {group.tasks.map((task) => (
              <TaskMark key={task.id} state={task.state} />
            ))}
          </span>
          <AgentChip agent={group.agent} />
        </button>
      </Tooltip>
      {open ? (
        <ul id={list} className="armada-plan-revision__tasks">
          {group.tasks.map((task) => (
            <TaskLine key={task.id} task={task} />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/** The plan in order: unchanged groups closed to a line, the changed group open with its changes marked where they stand. */
export function PlanRevisionView({ revision, unchanged }: { revision: PlanRevision; unchanged: Unchanged }) {
  return (
    <div className="armada-plan-revision">
      <ul className="armada-plan-revision__groups" aria-label="Plan">
        {revision.groups.map((group) =>
          changed(group) ? (
            <li key={group.id} className="armada-plan-revision__group">
              <div className="armada-plan-revision__head">
                <GroupTitle group={group} />
                <AgentChip agent={group.agent} />
              </div>
              <ul className="armada-plan-revision__tasks">
                {group.tasks.map((task) => (
                  <TaskLine key={task.id} task={task} />
                ))}
                {group.gone.map((text) => (
                  <GoneLine key={text} text={text} />
                ))}
              </ul>
            </li>
          ) : (
            <Quiet key={group.id} group={group} unchanged={unchanged} />
          ),
        )}
      </ul>
    </div>
  );
}
