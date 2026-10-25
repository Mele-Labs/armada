// Quick edits and notes on the plan, in place: rename by pressing a title, remove or note from the glyphs that show on hover, add a task or a
// group, drag a task to another group, change a group's subagent type. **None of it changes the plan.** Each lands as a mark
// (`plan-marks.ts`) and goes to the agent together when he presses Send changes, so the agent can answer each one. The same pieces are
// the review overlay's and the Plan panel's once the plan is approved.

import { useState } from "react";
import type { DragEvent, KeyboardEvent } from "react";
import { Bot, GraduationCap, MessageSquarePlus, MessageSquare, Pencil, Plus, Send, Trash2, Undo2, X } from "lucide-react";
import { Button, Input, Select, TaskMark, Textarea, Tooltip } from "@armada/components";
import type { PlanMark, WorkPlan } from "@armada/screens/src/draft/sessions";

import { useTeach } from "../teach";
import { addGroup, addNote, addTask, dropNote, move, remove, rename, setAgent, type Since, type ViewGroup, type ViewTask } from "./plan-marks";

/** The marks on the plan, and what changes them. `send` puts them to the agent as one message. */
export type PlanEditing = { id: string; plan: WorkPlan; marks: readonly PlanMark[]; set: (next: PlanMark[]) => void; send: () => void; /** Approves the plan, with the marks he made going in the same press. */ approve?: () => void };

/** The subagent types a group can be handed to. */
const AGENTS = ["rust-engineer", "bridge-engineer", "protocol-engineer", "fixture-author", "judge", "general-purpose"];

const key = (ed: PlanEditing, what: string) => `plan-note:${ed.id}:${what}`;

/** A name that is a button until pressed, then a field: Enter saves it and Esc, or leaving it, keeps the old one. */
function EditText({ value, was, label, onSave, className }: { value: string; was?: string | undefined; label: string; onSave: (to: string) => void; className?: string }) {
  const [draft, setDraft] = useState<string | undefined>();
  if (draft !== undefined) {
    const save = () => {
      setDraft(undefined);
      onSave(draft);
    };
    return (
      <Input
        aria-label={label}
        value={draft}
        autoFocus
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => setDraft(undefined)}
        onKeyDown={(event) => {
          if (event.key === "Enter") save();
          if (event.key === "Escape") {
            event.stopPropagation();
            setDraft(undefined);
          }
        }}
      />
    );
  }
  return (
    <span className="sv-edit">
      {was === undefined ? null : <s className="sv-edit__was">{was}</s>}
      <button type="button" className={`sv-edit__text${className === undefined ? "" : ` ${className}`}`} aria-label={`${label}: ${value}`} onClick={() => setDraft(value)}>
        {value}
      </button>
    </span>
  );
}

/** The glyphs a row shows on hover: a note pinned to it, Teach where the row offers it, and remove, which becomes undo once it is removed. */
function Tools({ what, removed, onNote, onRemove, onTeach }: { what: string; removed: boolean; onNote: () => void; onRemove: () => void; onTeach?: (() => void) | undefined }) {
  return (
    <span className="sv-edit__tools">
      <Tooltip label="Suggest">
        <Button variant="ghost" size="sm" iconOnly aria-label={`Suggest on ${what}`} onClick={onNote}>
          <MessageSquarePlus size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
      {onTeach === undefined ? null : (
        <Tooltip label="Teach">
          <Button variant="ghost" size="sm" iconOnly aria-label={`Teach on ${what}`} onClick={onTeach}>
            <GraduationCap size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      <Tooltip label={removed ? "Keep" : "Remove"}>
        <Button variant="ghost" size="sm" iconOnly aria-label={`${removed ? "Keep" : "Remove"} ${what}`} onClick={onRemove}>
          {removed ? <Undo2 size={16} strokeWidth={2} aria-hidden /> : <Trash2 size={16} strokeWidth={2} aria-hidden />}
        </Button>
      </Tooltip>
    </span>
  );
}

/** The one-line box a suggestion is written in. Enter pins it, Esc leaves it. */
function NoteBox({ about, label, onAdd, onClose }: { about: string; label: string; onAdd: (text: string) => void; onClose: () => void }) {
  const [text, setText] = useState("");
  const submit = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
    }
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      onAdd(text);
    }
  };
  return (
    <div className="sv-note-box">
      <Textarea aria-label={label} rows={1} keep={about} autoFocus value={text} onChange={(event) => setText(event.target.value)} onKeyDown={submit} />
      <Tooltip label="Close">
        <Button variant="ghost" size="sm" iconOnly aria-label="Close suggestion" onClick={onClose}>
          <X size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    </div>
  );
}

/** What he wrote, pinned to the thing it is about. */
function Notes({ notes, ed }: { notes: ViewGroup["notes"]; ed: PlanEditing }) {
  if (notes.length === 0) return null;
  return (
    <ul className="sv-pins" aria-label="Suggestions">
      {notes.map((note) => (
        <li key={note.id} className="sv-pin">
          <Tooltip label="Suggestion, not sent yet">
            <span role="img" aria-label="Suggestion" className="sv-pin__mark">
              <MessageSquare size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span>{note.text}</span>
          <Tooltip label="Take back">
            <Button variant="ghost" size="sm" iconOnly aria-label="Take back suggestion" onClick={() => ed.set(dropNote(ed.marks, note.id))}>
              <X size={12} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

/** Where the agent changed a group or task in the plan it recorded again. */
function SinceMark({ since }: { since: Since }) {
  const said = since.kind === "added" ? "Added by the agent" : `Renamed by the agent, was ${since.from}`;
  return (
    <Tooltip label={said}>
      <span className="sv-since" data-kind={since.kind} role="img" aria-label={said}>
        {since.kind === "added" ? <Plus size={12} strokeWidth={2} aria-hidden /> : null}
        {since.kind === "renamed" ? (
          <>
            <Pencil size={12} strokeWidth={2} aria-hidden />
            <s>{since.from}</s>
          </>
        ) : null}
      </span>
    </Tooltip>
  );
}

function TaskRow({ task, group, ed }: { task: ViewTask; group: ViewGroup; ed: PlanEditing }) {
  const [noting, setNoting] = useState(false);
  const teach = useTeach();
  const target = { group: task.home, task: task.id };
  return (
    <li
      className="sv-task sv-task--edit"
      data-state={task.state}
      data-removed={task.removed || undefined}
      data-added={task.added || undefined}
      draggable
      onDragStart={(event: DragEvent) => event.dataTransfer.setData("text/plain", task.id)}
    >
      <TaskMark state={task.state} />
      <span className="sv-task__body">
        <span className="sv-task__line">
          <EditText label="Task" value={task.text} was={task.was} onSave={(to) => ed.set(rename(ed.plan, ed.marks, target, to))} />
          {task.added === true ? (
            <Tooltip label="Added, not sent yet">
              <span className="sv-since" data-kind="added" role="img" aria-label="Added">
                <Plus size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          ) : null}
          {task.movedFrom === undefined ? null : <span className="sv-moved">from {task.movedFrom}</span>}
          {task.since === undefined ? null : <SinceMark since={task.since} />}
          <Tools what={`task ${task.text}`} removed={task.removed === true} onNote={() => setNoting(true)} onRemove={() => ed.set(remove(ed.marks, target))} onTeach={teach === undefined ? undefined : () => teach("task", task.text)} />
        </span>
        <Notes notes={task.notes} ed={ed} />
        {noting ? (
          <NoteBox
            about={key(ed, `${group.id}:${task.id}`)}
            label={`Suggest on ${task.text}`}
            onClose={() => setNoting(false)}
            onAdd={(text) => {
              ed.set(addNote(ed.marks, target, text));
              setNoting(false);
            }}
          />
        ) : null}
      </span>
    </li>
  );
}

/** A line to add a task to a group: a glyph until pressed, then a field. */
function AddTask({ group, ed }: { group: ViewGroup; ed: PlanEditing }) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  if (!open) {
    return (
      <Tooltip label="Add a task">
        <Button variant="ghost" size="sm" iconOnly aria-label={`Add a task to ${group.title}`} onClick={() => setOpen(true)}>
          <Plus size={16} strokeWidth={2} aria-hidden />
        </Button>
      </Tooltip>
    );
  }
  return (
    <Input
      aria-label={`New task in ${group.title}`}
      value={text}
      autoFocus
      onChange={(event) => setText(event.target.value)}
      onBlur={() => setOpen(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          setOpen(false);
        }
        if (event.key === "Enter") {
          ed.set(addTask(ed.marks, group.id, text));
          setText("");
          setOpen(false);
        }
      }}
    />
  );
}

/**
 * A group, edited in place: its subagent type, its title, the glyphs for a note and for removing it, then its paths and its tasks,
 * which take a drop from another group. Drawn open, as the overlay draws a group and as the panel does under a group's row.
 */
export function GroupEditor({ group, ed, head = true }: { group: ViewGroup; ed: PlanEditing; head?: boolean }) {
  const [noting, setNoting] = useState(false);
  // Without its head, the title and subagent type open behind a pencil, and stay open while either carries an edit.
  const [editing, setEditing] = useState(false);
  const headed = head || editing || group.was !== undefined || group.agentWas !== undefined;
  const target = { group: group.id };
  const drop = (event: DragEvent) => {
    event.preventDefault();
    const task = event.dataTransfer.getData("text/plain");
    if (task !== "") ed.set(move(ed.plan, ed.marks, task, group.id));
  };
  return (
    <div className="sv-edit-group" data-removed={group.removed || undefined} data-added={group.added || undefined} data-editing={(headed && !head) || undefined} onDragOver={(event) => event.preventDefault()} onDrop={drop}>
      {headed ? (
        <div className="sv-edit-group__head">
          <div className="sv-edit-group__name">
            <EditText label="Group" className="sv-group__title" value={group.title} was={group.was} onSave={(to) => ed.set(rename(ed.plan, ed.marks, target, to))} />
            {group.added === true ? (
              <Tooltip label="Added, not sent yet">
                <span className="sv-since" data-kind="added" role="img" aria-label="Added">
                  <Plus size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            ) : null}
            {group.since === undefined ? null : <SinceMark since={group.since} />}
            <Tools what={`group ${group.title}`} removed={group.removed === true} onNote={() => setNoting(true)} onRemove={() => ed.set(remove(ed.marks, target))} />
          </div>
          <Tooltip label="Subagent type">
            <span className="sv-group__agent">
              <Bot size={12} strokeWidth={2} aria-hidden />
              {group.agentWas === undefined ? null : <s className="sv-edit__was">{group.agentWas}</s>}
              <Select aria-label={`Subagent type of ${group.title}`} value={group.agent} onChange={(event) => ed.set(setAgent(ed.plan, ed.marks, group.id, event.target.value))}>
                {[...new Set([group.agent, ...AGENTS])].map((one) => (
                  <option key={one} value={one}>
                    {one}
                  </option>
                ))}
              </Select>
            </span>
          </Tooltip>
        </div>
      ) : (
        <div className="sv-edit-group__head">
          <span className="sv-edit__tools">
            <Tooltip label="Edit group">
              <Button variant="ghost" size="sm" iconOnly aria-label={`Edit group ${group.title}`} onClick={() => setEditing(true)}>
                <Pencil size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
            <Tools what={`group ${group.title}`} removed={group.removed === true} onNote={() => setNoting(true)} onRemove={() => ed.set(remove(ed.marks, target))} />
          </span>
        </div>
      )}
      <Notes notes={group.notes} ed={ed} />
      {noting ? (
        <NoteBox
          about={key(ed, group.id)}
          label={`Suggest on ${group.title}`}
          onClose={() => setNoting(false)}
          onAdd={(text) => {
            ed.set(addNote(ed.marks, target, text));
            setNoting(false);
          }}
        />
      ) : null}
      {group.paths.length === 0 ? null : (
        <ul className="sv-scope" aria-label="Paths">
          {group.paths.map((path) => (
            <li key={path}>
              <code>{path}</code>
            </li>
          ))}
        </ul>
      )}
      <ul className="sv-tasks">
        {group.tasks.map((task) => (
          <TaskRow key={task.id} task={task} group={group} ed={ed} />
        ))}
        {group.gone.map((text) => (
          <li key={text} className="sv-task sv-task--gone">
            <Tooltip label="Removed by the agent">
              <span role="img" aria-label="Removed by the agent" className="sv-since" data-kind="removed">
                <Trash2 size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
            <s>{text}</s>
          </li>
        ))}
      </ul>
      <AddTask group={group} ed={ed} />
    </div>
  );
}

/** A line to add a group: a button until pressed, then a field for its title. */
export function AddGroup({ ed }: { ed: PlanEditing }) {
  const [open, setOpen] = useState(false);
  const [text, setText] = useState("");
  if (!open) {
    return (
      <Button variant="ghost" size="sm" onClick={() => setOpen(true)}>
        <Plus size={16} strokeWidth={2} aria-hidden />
        Add group
      </Button>
    );
  }
  return (
    <Input
      aria-label="New group"
      value={text}
      autoFocus
      onChange={(event) => setText(event.target.value)}
      onBlur={() => setOpen(false)}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          setOpen(false);
        }
        if (event.key === "Enter") {
          ed.set(addGroup(ed.marks, text));
          setText("");
          setOpen(false);
        }
      }}
    />
  );
}

/** Send changes, where there are marks to send. */
export function SendChanges({ ed }: { ed: PlanEditing }) {
  if (ed.marks.length === 0) return null;
  return (
    <Button variant="secondary" size="sm" onClick={ed.send}>
      <Send size={16} strokeWidth={2} aria-hidden />
      Send changes
    </Button>
  );
}
