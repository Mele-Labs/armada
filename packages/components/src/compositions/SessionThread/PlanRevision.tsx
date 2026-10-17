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
 * A plan that is a later version of one already worked, drawn as what changed. Two takes of it: `diff` leads with the changes under their
 * group and folds everything else into one Unchanged part at the end; `folded` keeps the plan in order, unchanged groups closed to a line each
 * and the changed group open with its changes marked in place.
 */
export type PlanRevision = { take: "diff" | "folded"; groups: readonly RevisionGroup[] };

const changed = (group: RevisionGroup): boolean => group.since !== undefined || group.gone.length > 0 || group.tasks.some((task) => task.since !== undefined);

/** Standing in the open set for the one Unchanged part the `diff` take has. */
const ALL = "*";

/** What the key and the presses open: the one Unchanged part of the `diff` take, or each unchanged group of the `folded` one. */
export function useUnchanged(revision: PlanRevision | undefined) {
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const keys =
    revision === undefined
      ? []
      : revision.take === "diff"
        ? hasUnchanged(revision.groups)
          ? [ALL]
          : []
        : revision.groups.filter((group) => !changed(group)).map((group) => group.id);
  const every = keys.length > 0 && keys.every((key) => open.has(key));
  return {
    has: (key: string) => open.has(key),
    toggle: (key: string) =>
      setOpen((was) => {
        const next = new Set(was);
        if (!next.delete(key)) next.add(key);
        return next;
      }),
    /** Opens every unchanged part, or folds them all when all are open. */
    toggleAll: () => setOpen(every ? new Set() : new Set(keys)),
  };
}

export type Unchanged = ReturnType<typeof useUnchanged>;

const hasUnchanged = (groups: readonly RevisionGroup[]): boolean => groups.some((group) => !changed(group) || group.tasks.some((task) => task.since === undefined));

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

/** One task: the change it carries, its state, its text. `always` draws an open task's state; otherwise an open task leaves that cell empty. */
function TaskLine({ task, always }: { task: RevisionTask; always: boolean }) {
  const since = task.since;
  return (
    <li className="armada-plan-revision__task" data-kind={since?.kind}>
      {since === undefined ? <span aria-hidden /> : <ChangeMark kind={since.kind} />}
      {always || task.state !== "open" ? <TaskMark state={task.state} /> : <span aria-hidden />}
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

const Marks = ({ tasks }: { tasks: readonly RevisionTask[] }) => (
  <span className="armada-session-thread__marks">
    {tasks.map((task) => (
      <TaskMark key={task.id} state={task.state} />
    ))}
  </span>
);

/** The changes under their groups, then one Unchanged part: a line for each group nothing touched, and the other tasks of a group that changed. */
function Diff({ groups, unchanged }: { groups: readonly RevisionGroup[]; unchanged: Unchanged }) {
  const list = useId();
  const open = unchanged.has(ALL);
  const rest = groups.filter((group) => !changed(group) || group.tasks.some((task) => task.since === undefined));
  return (
    <div className="armada-plan-revision" data-take="diff">
      <ul className="armada-plan-revision__groups" aria-label="Changed since the last plan">
        {groups.filter(changed).map((group) => (
          <li key={group.id} className="armada-plan-revision__group">
            <div className="armada-plan-revision__head">
              <GroupTitle group={group} />
              <AgentChip agent={group.agent} />
            </div>
            <ul className="armada-plan-revision__tasks">
              {group.tasks.filter((task) => task.since !== undefined).map((task) => (
                <TaskLine key={task.id} task={task} always={false} />
              ))}
              {group.gone.map((text) => (
                <GoneLine key={text} text={text} />
              ))}
            </ul>
          </li>
        ))}
      </ul>
      {rest.length === 0 ? null : (
        <div className="armada-plan-revision__unchanged">
          <Tooltip asChild label={open ? "Hide unchanged" : "Show unchanged"} shortcut="U">
            <button type="button" className="armada-plan-revision__fold" aria-expanded={open} {...(open ? { "aria-controls": list } : {})} onClick={() => unchanged.toggle(ALL)}>
              {open ? <ChevronDown size={12} strokeWidth={2} aria-hidden /> : <ChevronRight size={12} strokeWidth={2} aria-hidden />}
              <span>Unchanged</span>
            </button>
          </Tooltip>
          {open ? (
            <ul id={list} className="armada-plan-revision__groups" aria-label="Unchanged">
              {rest.map((group) =>
                changed(group) ? (
                  <li key={group.id} className="armada-plan-revision__group">
                    <div className="armada-plan-revision__head">
                      <GroupTitle group={group} />
                      <AgentChip agent={group.agent} />
                    </div>
                    <ul className="armada-plan-revision__tasks">
                      {group.tasks.filter((task) => task.since === undefined).map((task) => (
                        <TaskLine key={task.id} task={task} always />
                      ))}
                    </ul>
                  </li>
                ) : (
                  <li key={group.id} className="armada-plan-revision__group">
                    <div className="armada-plan-revision__head">
                      <GroupTitle group={group} />
                      <Marks tasks={group.tasks} />
                      <AgentChip agent={group.agent} />
                    </div>
                  </li>
                ),
              )}
            </ul>
          ) : null}
        </div>
      )}
    </div>
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
          <Marks tasks={group.tasks} />
          <AgentChip agent={group.agent} />
        </button>
      </Tooltip>
      {open ? (
        <ul id={list} className="armada-plan-revision__tasks">
          {group.tasks.map((task) => (
            <TaskLine key={task.id} task={task} always />
          ))}
        </ul>
      ) : null}
    </li>
  );
}

/** The plan in order: unchanged groups closed to a line, the changed group open with its changes marked where they stand. */
function Folded({ groups, unchanged }: { groups: readonly RevisionGroup[]; unchanged: Unchanged }) {
  return (
    <div className="armada-plan-revision" data-take="folded">
      <ul className="armada-plan-revision__groups" aria-label="Plan">
        {groups.map((group) =>
          changed(group) ? (
            <li key={group.id} className="armada-plan-revision__group">
              <div className="armada-plan-revision__head">
                <GroupTitle group={group} />
                <AgentChip agent={group.agent} />
              </div>
              <ul className="armada-plan-revision__tasks">
                {group.tasks.map((task) => (
                  <TaskLine key={task.id} task={task} always />
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

export function PlanRevisionView({ revision, unchanged }: { revision: PlanRevision; unchanged: Unchanged }) {
  return revision.take === "diff" ? <Diff groups={revision.groups} unchanged={unchanged} /> : <Folded groups={revision.groups} unchanged={unchanged} />;
}
