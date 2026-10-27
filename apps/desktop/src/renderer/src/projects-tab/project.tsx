// The project open, or a repository's own tasks: the field that adds a task under its head, then its rows as cards, tasks
// he wrote and issues he linked, in his order. A card is its state's glyph and its title, with the issues a task carries
// beside the title; on the right, in columns that hold their place, a done ask's two answers, the Job holding it as a
// chip, and its acts: Dispatch, add a subtask, delete. A task's subtasks stand under it as smaller cards, one level deep.
// A ⌘-click on a card selects it and a ⇧-click selects the range from the card last clicked. A card is dragged to move it
// among its own list, as ⌥↑ and ⌥↓ do. Links are made by the tags written in the add field and in a title being edited.

import { useEffect, useRef, useState } from "react";
import { Check, CircleDashed, CircleDot, File, GitPullRequest, ListTree, MessageCircleQuestion, Send, SquareTerminal, Ticket, Trash2, X, type LucideIcon } from "lucide-react";
import { Button, Kbd, PullRequestChip, RepositoryChip, SessionMark, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useReorder } from "../cockpit/deck/reorder";
import { JobMark } from "../issues-tab/marks";
import { useSessions } from "../sessions-draft";
import type { Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import type { Holder, Linked, LinkedPull, Project, ProjectRow, RowState, TaskRow } from "./model";
import { rowState, useProjects, useProjectsStore } from "./store";
import { useTagField } from "./tagging";

export type Hosts = { onOpenSession: (id: string) => void; onOpenLink: (address: string) => void; onDispatch: (project: Project, rows: readonly ProjectRow[]) => void };

/** Each state's glyph and the word its tooltip names it by. An issue's open is its ticket; closed is its tracker's tick. */
function markOf(row: ProjectRow, state: RowState): { Glyph: LucideIcon; said: string } {
  if (row.kind === "issue") return state === "closed" ? { Glyph: Check, said: "Closed" } : { Glyph: Ticket, said: "Open" };
  if (state === "held") return { Glyph: CircleDot, said: "In a Job" };
  if (state === "asking") return { Glyph: MessageCircleQuestion, said: "Done?" };
  if (state === "done") return { Glyph: Check, said: "Done" };
  return { Glyph: CircleDashed, said: "Open" };
}

/** What a row is called: an issue by its reference and title. */
export const rowLabel = (row: ProjectRow): string => (row.kind === "issue" ? [row.reference, row.title].filter((one) => one !== "").join(" ") : row.title);

/** The cards a drag moves among: this project's, each holding its `[data-row]`. */
const CARDS = ".projects-item-card[data-movable]";

/** With nothing open the pane is empty: an empty slot stays empty. */
export function ProjectPane({ band, project, ...hosts }: Hosts & { band: Band | undefined; project: Project | undefined }) {
  if (band === undefined || project === undefined) return <section className="deck-pane projects-pane" aria-label="Project" tabIndex={-1} />;
  return <Pane key={project.id} band={band} project={project} {...hosts} />;
}

/** What Dispatch sends from a project: the rows selected, a parent with its subtasks, and a subtask on its own where its parent is not. */
export const chosenIn = (project: Project, selected: readonly string[]): ProjectRow[] =>
  project.rows.flatMap((row): ProjectRow[] =>
    selected.includes(row.id) ? [row] : row.kind === "task" ? (row.subtasks ?? []).filter((one) => selected.includes(one.id)) : [],
  );

/**
 * The cards of any list: `item` draws a row as a card with its subtasks under it, moved among the rows of its own project by a
 * drag. `from` is a small line above the card naming where it stands, for a list that mixes projects.
 */
export function useCards({ onOpenSession, onOpenLink, onDispatch }: Hosts) {
  const store = useProjectsStore()!;
  const { selected, subtasking, editing } = useProjects();
  const sessions = useSessions();
  const { dragging, grab, swallowed } = useReorder((key, target, after) => store.place(key, target, after), CARDS);
  const item = (project: Project, row: ProjectRow, list: string, parent?: TaskRow, from?: string): React.ReactNode => (
    <li
      key={row.id}
      className="projects-item-card"
      data-movable={list}
      data-lifted={dragging?.key === row.id || undefined}
      data-drop={dragging?.drop?.key === row.id ? (dragging.drop.after ? "after" : "before") : undefined}
    >
      {from === undefined ? null : <span className="projects-item-card__from">{from}</span>}
      <Card
        row={row}
        sub={parent !== undefined}
        project={project}
        adding={subtasking === row.id}
        editing={editing === row.id}
        state={rowState(row)}
        sessions={sessions}
        selected={selected.includes(row.id) || (parent !== undefined && selected.includes(parent.id))}
        onOpenSession={onOpenSession}
        onOpenLink={onOpenLink}
        onDispatch={onDispatch}
        onGrab={(event) => grab(event, row.id, list)}
        swallowed={swallowed}
      />
      {parent === undefined && row.kind === "task" && (row.subtasks ?? []).length > 0 ? (
        <ul className="projects-subtasks" aria-label={`Subtasks of ${row.title}`}>
          {(row.subtasks ?? []).map((one) => item(project, one, row.id, row))}
        </ul>
      ) : null}
    </li>
  );
  return { item, dragging };
}

function Pane({ band, project, ...hosts }: Hosts & { band: Band; project: Project }) {
  const { selected } = useProjects();
  const { onDispatch } = hosts;
  const chosen = chosenIn(project, selected);
  const { item, dragging } = useCards(hosts);
  return (
    <section className="deck-pane projects-pane" aria-label={project.own === true ? `Tasks of ${band.label}` : `Project ${project.name}`} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{project.own === true ? band.label : project.name}</h2>
        {project.own === true ? null : <RepositoryChip name={band.label} />}
        <span className="deck-pane__acts">
          <Tooltip label={project.own === true ? "Go to the tasks" : "Go to the project"} shortcut={PROJECTS_KEYS.rows}>
            <Kbd>{PROJECTS_KEYS.rows}</Kbd>
          </Tooltip>
          <Tooltip label="Dispatch the rows selected to one Job" shortcut={PROJECTS_KEYS.dispatch}>
            <Button variant="secondary" size="sm" data-dispatch disabled={chosen.length === 0} onClick={() => onDispatch(project, chosen)}>
              Dispatch
            </Button>
          </Tooltip>
        </span>
      </header>
      <div className="projects-top">
        <AddTask project={project} />
      </div>
      <ul className="projects-rows" aria-label="Rows" data-reordering={dragging === undefined ? undefined : ""}>
        {project.rows.map((row) => item(project, row, project.id))}
      </ul>
    </section>
  );
}

type CardProps = {
  row: ProjectRow;
  /** A subtask: its parent's selection takes it, and it holds no subtasks or issues of its own. */
  sub: boolean;
  project: Project;
  /** Its subtask field is up, inside the card. */
  adding: boolean;
  /** Its title is being edited in place. */
  editing: boolean;
  state: RowState;
  sessions: readonly Session[];
  selected: boolean;
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
  onDispatch: Hosts["onDispatch"];
  onGrab: (event: React.PointerEvent<HTMLElement>) => void;
  swallowed: () => boolean;
};

function Card({ row, sub, project, adding, editing, state, sessions, selected, onOpenSession, onOpenLink, onDispatch, onGrab, swallowed }: CardProps) {
  const store = useProjectsStore()!;
  const { Glyph, said } = markOf(row, state);
  const label = rowLabel(row);
  // A pull request Fleet listed has its number only: where it is on the forge is read when it is pressed.
  const openPull = (one: LinkedPull) => {
    if (one.url !== "") return onOpenLink(one.url);
    void window.armada.getPullRequest(project.manifestId, one.number).then((read) => read.ok && onOpenLink(read.value.address));
  };
  const interactive = (target: EventTarget) => target instanceof Element && target.closest("button, a, textarea, input") !== null;
  const picking = (event: React.MouseEvent | React.PointerEvent) => event.metaKey || event.ctrlKey || event.shiftKey;
  return (
    <div
      className="projects-row"
      data-row={row.id}
      data-state={state}
      data-sub={sub || undefined}
      data-asking={state === "asking" || undefined}
      data-selected={selected || undefined}
      tabIndex={0}
      role="group"
      aria-label={label}
      // A ⌘ or ⇧ press picks cards, so it never starts a drag, nor selects the words under it.
      onPointerDown={(event) => !interactive(event.target) && !picking(event) && onGrab(event)}
      onMouseDown={(event) => {
        if (!picking(event) || interactive(event.target)) return;
        event.preventDefault();
        event.currentTarget.focus();
      }}
      onClickCapture={(event) => swallowed() && (event.preventDefault(), event.stopPropagation())}
      onClick={(event) => {
        if (interactive(event.target)) return;
        if (event.metaKey || event.ctrlKey) store.select(row.id);
        else if (event.shiftKey) store.selectTo(row.id);
        else store.anchorAt(row.id);
      }}
    >
      {row.kind === "task" ? (
        <Tooltip label={said} shortcut={PROJECTS_KEYS.tick} asChild>
          <button type="button" tabIndex={-1} className="projects-row__mark" data-state={state} role="checkbox" aria-checked={row.done} aria-label={`${said}: ${row.title}`} onClick={() => store.tick(row.id)}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      ) : (
        <Tooltip label={said} asChild>
          <span className="projects-row__mark" data-state={state} role="img" aria-label={said}>
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      <span className="projects-row__title">
        {row.kind === "issue" ? <span className="projects-row__reference">{row.reference}</span> : null}
        {editing && row.kind === "task" ? <EditTitle row={row} manifestId={project.manifestId} /> : <span className="projects-row__words">{row.title}</span>}
        {row.kind === "task" && !sub ? <IssueChips issues={row.issues ?? []} /> : null}
      </span>
      {/* The done ask's answers, in a column of their own; a card with no ask leaves it to the title. */}
      {state === "asking" ? (
        <span className="projects-row__ask">
          <Tooltip label="Mark done" shortcut={capOf(PROJECTS_KEYS.markDone)} asChild>
            <Button variant="secondary" size="sm" tabIndex={-1} data-mark-done onClick={() => store.settle(row.id, true)}>
              Mark done
            </Button>
          </Tooltip>
          <Tooltip label="Keep open" shortcut={capOf(PROJECTS_KEYS.keepOpen)} asChild>
            <Button variant="ghost" size="sm" tabIndex={-1} data-keep-open onClick={() => store.settle(row.id, false)}>
              Keep open
            </Button>
          </Tooltip>
        </span>
      ) : null}
      {/* The acts, always drawn, each in its own column at the card's right edge; a card without one keeps its place. */}
      <span className="projects-row__acts">
        {row.kind === "task" && !sub ? (
          <>
            <Tooltip label="Dispatch" shortcut={PROJECTS_KEYS.dispatch} asChild>
              <button type="button" tabIndex={-1} className="projects-row__act" data-dispatch-task aria-label={`Dispatch ${row.title}`} onClick={() => onDispatch(project, [row])}>
                <Send size={16} strokeWidth={2} aria-hidden />
              </button>
            </Tooltip>
            <Tooltip label="Add a subtask" shortcut={PROJECTS_KEYS.addSubtask} asChild>
              <button type="button" tabIndex={-1} className="projects-row__act" data-add-subtask aria-label={`Add a subtask to ${row.title}`} onClick={() => store.subtasking(row.id)}>
                <ListTree size={16} strokeWidth={2} aria-hidden />
              </button>
            </Tooltip>
          </>
        ) : (
          <>
            <span className="projects-row__act" aria-hidden="true" />
            <span className="projects-row__act" aria-hidden="true" />
          </>
        )}
        <Tooltip label={row.kind === "task" ? "Delete" : "Unlink"} shortcut={PROJECTS_KEYS.remove} asChild>
          <button type="button" tabIndex={-1} className="projects-row__act" data-remove aria-label={`${row.kind === "task" ? "Delete" : "Unlink"} ${label}`} onClick={() => store.remove(row.id)}>
            <Trash2 size={16} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      </span>
      {adding && row.kind === "task" ? (
        <span className="projects-row__subadd">
          <AddTask project={project} parent={row} />
        </span>
      ) : null}
      {/* The Job holding it and the pull requests it carries, on a line of their own under the title, each with its own Unlink. */}
      {row.job === undefined && (row.kind !== "task" || (row.pulls ?? []).length === 0) ? null : (
        <span className="projects-row__holder">
          {row.job === undefined ? null : (
            <span className="projects-row__linked">
              <JobChip job={row.job} sessions={sessions} onOpen={onOpenSession} />
              {row.kind === "task" ? <Unlink said={`Unlink ${row.job.title}`} onUnlink={() => store.unlinkJob(row.id)} /> : null}
            </span>
          )}
          {row.kind === "task"
            ? (row.pulls ?? []).map((one) => (
                <span key={one.number} className="projects-row__linked">
                  <PullRequestChip framed number={one.number} title={one.title} state={one.state} tabIndex={-1} onOpen={() => openPull(one)} />
                  <Unlink pull said={`Unlink #${one.number}`} onUnlink={() => store.unlinkPull(row.id, one.number)} />
                </span>
              ))
            : null}
        </span>
      )}
    </div>
  );
}

/** A chip's own small x. */
function Unlink({ said, pull, onUnlink }: { said: string; pull?: true; onUnlink: () => void }) {
  return (
    <Tooltip label="Unlink" shortcut={capOf(pull === true ? PROJECTS_KEYS.unlinkPull : PROJECTS_KEYS.unlinkJob)} asChild>
      <button type="button" tabIndex={-1} className="projects-row__unlink" data-unlink-pull={pull} aria-label={said} onClick={onUnlink}>
        <X size={12} strokeWidth={2} aria-hidden />
      </button>
    </Tooltip>
  );
}

/** The issues a task carries, each by its reference; the tooltip names it. */
function IssueChips({ issues }: { issues: readonly Linked[] }) {
  if (issues.length === 0) return null;
  return (
    <span className="projects-row__issues">
      {issues.map((one) => (
        <Tooltip key={one.reference} label={one.title === "" ? one.reference : one.title} asChild>
          <span className="projects-row__issue" data-state={one.state} role="img" aria-label={[one.reference, one.title].filter((word) => word !== "").join(" ")}>
            {one.reference}
          </span>
        </Tooltip>
      ))}
    </span>
  );
}

/** The Job holding a row, as a chip: its live state's glyph and its title; a press opens it. One that has ended reads by the title it was sent with. */
function JobChip({ job, sessions, onOpen }: { job: Holder; sessions: readonly Session[]; onOpen: (id: string) => void }) {
  const session = sessions.find((one) => one.id === job.id);
  const title = session?.title ?? job.title;
  return (
    <Tooltip label="Open the Job" shortcut={PROJECTS_KEYS.openJob} asChild>
    <button type="button" tabIndex={-1} className="projects-row__job" data-job aria-label={`Open ${title}`} onClick={() => onOpen(job.id)}>
      {session === undefined ? <SessionMark state="ended" said={`${job.title}: Ended`} /> : <JobMark session={session} />}
      <span className="projects-row__job-title">{title}</span>
    </button>
    </Tooltip>
  );
}

/** What the composer's buttons put at the caret: `@` opens the Jobs and files, `#` the issues and pull requests. */
const TOOLS = [
  { Glyph: SquareTerminal, said: "Job", trigger: "@" },
  { Glyph: File, said: "File", trigger: "@" },
  { Glyph: Ticket, said: "Issue", trigger: "#" },
  { Glyph: GitPullRequest, said: "Pull request", trigger: "#" },
] as const;

/**
 * A task, written under the project's head: Enter adds it at the top of the list and the field stays for the next, ⇧Enter is
 * a new line. The top field is one line at rest and grows into a composer, a few lines with a button each for a Job, a file,
 * an issue and a pull request, while it has the focus or holds words; with no words, Esc or leaving it folds it back. Under a
 * task (`parent`), inside its card, the field adds its subtasks the same way, always one line, and Enter on it empty puts it
 * away. A draft is kept across leaving the panel.
 */
export function AddTask({ project, parent }: { project: Project; parent?: TaskRow }) {
  const store = useProjectsStore()!;
  const keep = parent === undefined ? `projects:task:${project.id}` : `projects:subtask:${parent.id}`;
  const [title, setTitle] = useState("");
  const [focused, setFocused] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const tags = useTagField(project.manifestId, title, setTitle);
  const composer = parent === undefined && (focused || title !== "");
  // A press that takes the focus elsewhere folds the field once the press is over, so the card pressed is still under it.
  const pressing = useRef(false);
  const folding = useRef(false);
  useEffect(() => {
    if (!focused) return;
    const down = () => void (pressing.current = true);
    const up = () => {
      pressing.current = false;
      if (!folding.current) return;
      folding.current = false;
      window.setTimeout(() => setFocused(false), 0);
    };
    window.addEventListener("pointerdown", down, true);
    window.addEventListener("pointerup", up, true);
    window.addEventListener("pointercancel", up, true);
    return () => {
      window.removeEventListener("pointerdown", down, true);
      window.removeEventListener("pointerup", up, true);
      window.removeEventListener("pointercancel", up, true);
    };
  }, [focused]);
  const add = () => {
    if (title.trim() === "") {
      if (parent === undefined) return;
      store.subtasking(undefined);
      return refocusRow(parent.id);
    }
    const written = tags.resolve(title);
    if (parent === undefined) store.addTask(project.id, written.title, written.links);
    else store.addSubtask(parent.id, written.title, written.links);
    forgetKept(keep);
    setTitle("");
  };
  const said = parent === undefined ? "Add a task" : `Add a subtask to ${parent.title}`;
  return (
    <form
      ref={form}
      className="projects-add"
      aria-label={said}
      data-sub={parent === undefined ? undefined : ""}
      data-composer={composer || undefined}
      onSubmit={(event) => {
        event.preventDefault();
        add();
      }}
      onFocus={() => {
        folding.current = false;
        setFocused(true);
      }}
      onBlur={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
        if (pressing.current) folding.current = true;
        else setFocused(false);
      }}
    >
      <div className="projects-add__box">
        <Textarea
          rows={composer ? 3 : 1}
          keep={keep}
          autoFocus={parent !== undefined}
          aria-label={said}
          placeholder={parent === undefined ? "Add a task" : "Add a subtask"}
          value={title}
          {...tags.field}
          onKeyDown={(event) => {
            if (tags.field.onKeyDown(event)) return;
            if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
              event.preventDefault();
              add();
            }
          }}
        />
        {tags.menu}
        {composer ? (
          <div className="projects-add__tools" role="group" aria-label="Tag">
            {TOOLS.map(({ Glyph, said: name, trigger }) => (
              <Tooltip key={name} label={name} shortcut={trigger} asChild>
                <button
                  type="button"
                  className="projects-add__tool"
                  aria-label={name}
                  // The field keeps the focus, so the caret the trigger lands at is the one it had.
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => {
                    const box = form.current?.querySelector("textarea");
                    if (box) tags.insert(box, trigger);
                  }}
                >
                  <Glyph size={16} strokeWidth={2} aria-hidden />
                </button>
              </Tooltip>
            ))}
          </div>
        ) : null}
      </div>
      {parent === undefined ? (
        <Tooltip label="Add a task" shortcut={PROJECTS_KEYS.addTask}>
          <Kbd>{PROJECTS_KEYS.addTask}</Kbd>
        </Tooltip>
      ) : null}
    </form>
  );
}

/** A task's title, edited in place. Enter keeps it; Esc, which the tab's keys read, leaves it as it was. */
function EditTitle({ row, manifestId }: { row: TaskRow; manifestId: string }) {
  const store = useProjectsStore()!;
  const keep = `projects:edit:${row.id}`;
  const [title, setTitle] = useState(row.title);
  const tags = useTagField(manifestId, title, setTitle);
  return (
    <span className="projects-tag">
    <Textarea
      rows={1}
      autoFocus
      keep={keep}
      aria-label={`Title of ${row.title}`}
      value={title}
      {...tags.field}
      onKeyDown={(event) => {
        if (tags.field.onKeyDown(event)) return;
        if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
          event.preventDefault();
          forgetKept(keep);
          const written = tags.resolve(title);
          if (written.title === "") store.editing(undefined);
          else {
            store.retitle(row.id, written.title);
            for (const issue of written.links.issues ?? []) store.linkToTask(row.id, issue);
            for (const pull of written.links.pulls ?? []) store.linkPull(row.id, pull);
            if (written.links.job !== undefined) store.linkJob(row.id, written.links.job);
          }
          refocusRow(row.id);
        }
      }}
    />
    {tags.menu}
    </span>
  );
}

/** The card a row is drawn as, once the list has drawn again. */
export const refocusRow = (rowId: string): void => void window.requestAnimationFrame(() => document.querySelector<HTMLElement>(`.projects-row[data-row="${CSS.escape(rowId)}"]`)?.focus());
