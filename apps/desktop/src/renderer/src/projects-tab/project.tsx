// The cards of the Projects tab and the field that adds a task. A card is a neutral tile: its state's glyph and its title, with
// the issues a task carries beside the title and its repository as a quiet chip where the list holds more than one; on the right,
// in columns that hold their place, a done ask's two answers and its acts: Dispatch (a subtask has it too), add a subtask, delete.
// The Job holding it and the pull requests it carries stand on a line under the title. A task's subtasks stand under it as smaller cards, one level
// deep. A ⌘-click on a card selects it and a ⇧-click selects the range from the card last clicked, in any repository. A card is
// dragged to move it among its own project's, as ⌥↑ and ⌥↓ do. Links are made by the tags written in the add field and in a
// title being edited. The add field names the repository the task is in and, optionally, its project, and holds the key that goes to it.

import { useEffect, useRef, useState } from "react";
import { Check, CircleDashed, CircleDot, File, Folder, GitPullRequest, ListTree, MessageCircleQuestion, Send, SquareTerminal, Ticket, Trash2, X, type LucideIcon } from "lucide-react";
import { Button, DropdownMenu, Kbd, PullRequestChip, Readout, RepositoryChip, SessionMark, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useReorder } from "../cockpit/deck/reorder";
import { JobMark } from "../issues-tab/marks";
import { useSessions } from "../sessions-draft";
import type { Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import type { Holder, Linked, LinkedPull, ProjectRow, RowState, TaskRow } from "./model";
import { rowState, useProjects, useProjectsStore } from "./store";
import { NO_PROJECT, shown } from "./views";
import { useTagField } from "./tagging";

export type Hosts = { onOpenSession: (id: string) => void; onOpenLink: (address: string) => void; onDispatch: (rows: readonly ProjectRow[]) => void };

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

/** What Dispatch sends from a list: the rows selected, a parent with its subtasks, and a subtask on its own where its parent is not. */
export const chosenOf = (rows: readonly ProjectRow[], selected: readonly string[]): ProjectRow[] =>
  rows.flatMap((row): ProjectRow[] => (selected.includes(row.id) ? [row] : row.kind === "task" ? (row.subtasks ?? []).filter((one) => selected.includes(one.id)) : []));

/** How a list draws its cards. */
export type CardOptions = {
  /** The cards are moved among their own project's by a drag. */
  movable: boolean;
  /** Each card names its repository: the list holds more than one. */
  many: boolean;
};

/**
 * The cards of any list: `item` draws a row as a card with its subtasks under it, moved among the rows of its own project by a
 * drag where the list is a project's pane. `labels` are the repositories' names, by Manifest id.
 */
export function useCards({ onOpenSession, onOpenLink, onDispatch }: Hosts, labels: ReadonlyMap<string, string>) {
  const store = useProjectsStore()!;
  const { selected, subtasking, editing, showDone } = useProjects();
  const sessions = useSessions();
  const { dragging, grab, swallowed } = useReorder((key, target, after) => store.place(key, target, after), CARDS);
  const item = (row: ProjectRow, list: string, options: CardOptions, parent?: TaskRow): React.ReactNode => (
    <li
      key={row.id}
      className="projects-item-card"
      data-movable={options.movable ? list : undefined}
      data-lifted={dragging?.key === row.id || undefined}
      data-drop={dragging?.drop?.key === row.id ? (dragging.drop.after ? "after" : "before") : undefined}
    >
      {/* A subtask a search found without its parent names the parent, faintly, above it. */}
      {parent !== undefined && list !== parent.id ? (
        <p className="projects-item-card__parent" aria-label={`Subtask of ${parent.title}`}>
          {parent.title}
        </p>
      ) : null}
      <Card
        row={row}
        sub={parent !== undefined}
        repository={options.many && parent === undefined ? labels.get(row.manifestId) : undefined}
        movable={options.movable}
        adding={subtasking === row.id}
        editing={editing === row.id}
        state={rowState(row)}
        sessions={sessions}
        selected={selected.includes(row.id) || (parent !== undefined && selected.includes(parent.id))}
        onOpenSession={onOpenSession}
        onOpenLink={onOpenLink}
        onDispatch={onDispatch}
        onGrab={(event) => options.movable && grab(event, row.id, list)}
        swallowed={swallowed}
      />
      {parent === undefined && row.kind === "task" && (row.subtasks ?? []).some((one) => shown(one, showDone)) ? (
        <ul className="projects-subtasks" aria-label={`Subtasks of ${row.title}`}>
          {(row.subtasks ?? []).filter((one) => shown(one, showDone)).map((one) => item(one, row.id, options, row))}
        </ul>
      ) : null}
    </li>
  );
  return { item, dragging };
}

type CardProps = {
  row: ProjectRow;
  /** A subtask: its parent's selection takes it, and it holds no subtasks or issues of its own. */
  sub: boolean;
  /** The repository's name, drawn as a chip: the list holds more than one. */
  repository: string | undefined;
  movable: boolean;
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

function Card({ row, sub, repository, movable, adding, editing, state, sessions, selected, onOpenSession, onOpenLink, onDispatch, onGrab, swallowed }: CardProps) {
  const store = useProjectsStore()!;
  const { Glyph, said } = markOf(row, state);
  const label = rowLabel(row);
  // A pull request Fleet listed has its number only: where it is on the forge is read when it is pressed.
  const openPull = (one: LinkedPull) => {
    if (one.url !== "") return onOpenLink(one.url);
    void window.armada.getPullRequest(row.manifestId, one.number).then((read) => read.ok && onOpenLink(read.value.address));
  };
  const interactive = (target: EventTarget) => target instanceof Element && target.closest("button, a, textarea, input") !== null;
  const picking = (event: React.MouseEvent | React.PointerEvent) => event.metaKey || event.ctrlKey || event.shiftKey;
  return (
    <div
      className="projects-row"
      data-row={row.id}
      data-state={state}
      data-sub={sub || undefined}
      data-fixed={movable ? undefined : ""}
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
        {row.kind === "issue" ? <Readout fixed className="projects-row__reference">{row.reference}</Readout> : null}
        {editing && row.kind === "task" ? <EditTitle row={row} /> : <span className="projects-row__words">{row.title}</span>}
        {row.kind === "task" && !sub ? <IssueChips issues={row.issues ?? []} /> : null}
        {repository === undefined ? null : <RepositoryChip name={repository} />}
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
        {row.kind === "task" ? (
          <Tooltip label="Dispatch" shortcut={PROJECTS_KEYS.dispatch} asChild>
            <button type="button" tabIndex={-1} className="projects-row__act" data-dispatch-task aria-label={`Dispatch ${row.title}`} onClick={() => onDispatch([row])}>
              <Send size={16} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
        ) : (
          <span className="projects-row__act" aria-hidden="true" />
        )}
        {row.kind === "task" && !sub ? (
          <Tooltip label="Add a subtask" shortcut={PROJECTS_KEYS.addSubtask} asChild>
            <button type="button" tabIndex={-1} className="projects-row__act" data-add-subtask aria-label={`Add a subtask to ${row.title}`} onClick={() => store.subtasking(row.id)}>
              <ListTree size={16} strokeWidth={2} aria-hidden />
            </button>
          </Tooltip>
        ) : (
          <span className="projects-row__act" aria-hidden="true" />
        )}
        <Tooltip label={row.kind === "task" ? "Delete" : "Unlink"} shortcut={PROJECTS_KEYS.remove} asChild>
          <button type="button" tabIndex={-1} className="projects-row__act" data-remove aria-label={`${row.kind === "task" ? "Delete" : "Unlink"} ${label}`} onClick={() => store.remove(row.id)}>
            <Trash2 size={16} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      </span>
      {adding && row.kind === "task" ? (
        <span className="projects-row__subadd">
          <AddTask parent={row} />
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
                  <PullRequestChip number={one.number} title={one.title} state={one.state} tabIndex={-1} onOpen={() => openPull(one)} />
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
          <Readout className="projects-row__issue" data-state={one.state} role="img" aria-label={[one.reference, one.title].filter((word) => word !== "").join(" ")}>
            {one.reference}
          </Readout>
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

/** Where the top field writes a task: the pane fixes what it is open on, and the field offers the rest. */
export type AddScope = {
  /** The draft's name, so each pane keeps its own words. */
  key: string;
  /** The repository the pane is open on: fixed. */
  repo?: string;
  /** The project the pane is open on: fixed. */
  project?: string;
};

/**
 * A task, written at the head of a pane: Enter adds it at the top of the list and the field stays for the next, ⇧Enter is a new
 * line. The top field is one line at rest and grows into a composer, a few lines with a button each for a Job, a file, an issue
 * and a pull request, and two chips: the repository the task is in, which it must name, and the project, which it may. Both are
 * fixed by the pane that fixes them and a menu otherwise. The repository starts as the one open, else the one the last task went
 * to, else the first; the project as the one open, else none. While it has the focus or holds words the composer stays up; with no
 * words, Esc or leaving it folds it back. Under a task (`parent`), inside its card, the field adds its subtasks the same way, always
 * one line, in the task's own repository, and Enter on it empty puts it away. A draft is kept across leaving the panel.
 */
export function AddTask({ bands, scope, parent }: { bands?: readonly Band[]; scope?: AddScope; parent?: TaskRow }) {
  const store = useProjectsStore()!;
  const { projects, lastRepo } = useProjects();
  const [pickedRepo, pickRepo] = useState<string | undefined>(undefined);
  const [pickedProject, pickProject] = useState<string | undefined>(undefined);
  const known = (id: string | undefined) => (id !== undefined && (bands ?? []).some((band) => band.manifestId === id) ? id : undefined);
  const repo = parent?.manifestId ?? known(scope?.repo) ?? known(pickedRepo) ?? known(lastRepo) ?? bands?.[0]?.manifestId ?? "";
  const project = scope?.project ?? (projects.some((one) => one.id === pickedProject) ? pickedProject : undefined);
  const keep = parent === undefined ? `projects:task:${scope?.key ?? ""}` : `projects:subtask:${parent.id}`;
  const [title, setTitle] = useState("");
  const [focused, setFocused] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const tags = useTagField(repo, title, setTitle);
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
    if (parent === undefined) store.addTask(repo, project, written.title, written.links);
    else store.addSubtask(parent.id, written.title, written.links);
    forgetKept(keep);
    setTitle("");
    pickRepo(undefined);
  };
  const said = parent === undefined ? "Add a task" : `Add a subtask to ${parent.title}`;
  const repoName = (bands ?? []).find((band) => band.manifestId === repo)?.label ?? repo;
  const projectName = projects.find((one) => one.id === project)?.name;
  // The key that goes to the field, inside it: at the right of the line at rest, at the right of the tool row as a composer.
  const hint =
    parent === undefined ? (
      <span className="projects-add__hint">
        <Tooltip label="Add a task" shortcut={PROJECTS_KEYS.addTask}>
          <Kbd>{PROJECTS_KEYS.addTask}</Kbd>
        </Tooltip>
      </span>
    ) : null;
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
        {composer ? null : hint}
        {composer ? (
          <div className="projects-add__tools">
            <div className="projects-add__tags" role="group" aria-label="Tag">
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
            <div className="projects-add__where" role="group" aria-label="Where">
              {scope?.repo !== undefined ? (
                <RepositoryChip name={repoName} />
              ) : (
                <DropdownMenu
                  align="start"
                  triggerLabel={repoName}
                  triggerAriaLabel="Repository"
                  triggerTooltip="Repository"
                  entries={(bands ?? []).map((band) => ({ kind: "item" as const, id: band.manifestId, label: band.label, selected: band.manifestId === repo }))}
                  onSelect={pickRepo}
                />
              )}
              {scope?.project !== undefined ? (
                <Tooltip label="Project">
                  <span className="projects-add__fixed" role="img" aria-label={`Project ${projectName ?? ""}`}>
                    <Folder size={12} strokeWidth={2} aria-hidden />
                    {projectName}
                  </span>
                </Tooltip>
              ) : (
                <DropdownMenu
                  align="start"
                  triggerLabel={projectName ?? NO_PROJECT}
                  triggerAriaLabel="Project"
                  triggerTooltip="Project"
                  entries={[
                    { kind: "item" as const, id: "", label: NO_PROJECT, selected: project === undefined },
                    ...projects.map((one) => ({ kind: "item" as const, id: one.id, label: one.name, selected: one.id === project })),
                  ]}
                  onSelect={(id) => pickProject(id === "" ? undefined : id)}
                />
              )}
            </div>
            {hint}
          </div>
        ) : null}
      </div>
    </form>
  );
}

/** A task's title, edited in place. Enter keeps it; Esc, which the tab's keys read, leaves it as it was. */
function EditTitle({ row }: { row: TaskRow }) {
  const store = useProjectsStore()!;
  const keep = `projects:edit:${row.id}`;
  const [title, setTitle] = useState(row.title);
  const tags = useTagField(row.manifestId, title, setTitle);
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
