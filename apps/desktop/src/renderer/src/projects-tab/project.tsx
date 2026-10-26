// The project open: the field that adds a task under its head, then its rows as cards, tasks he wrote and issues he
// linked, in his order. A card is its select control, its state's glyph and its title, with the issues a task carries
// beside the title; on the right, in columns that hold their place, a done ask's two answers, the Job holding it as a
// chip, and its acts. A task's subtasks stand under it as smaller cards, one level deep. A card is dragged to move it
// among its own list, as ⌥↑ and ⌥↓ do.

import { useEffect, useRef, useState } from "react";
import { Check, CircleDashed, CircleDot, Link, ListTree, MessageCircleQuestion, Ticket, Trash2, X, GitPullRequest, SquareTerminal, type LucideIcon } from "lucide-react";
import { Button, Kbd, PullRequestChip, RepositoryChip, SelectMark, SessionMark, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { useReorder } from "../cockpit/deck/reorder";
import { JobMark, jobTitle } from "../issues-tab/marks";
import { useIssues, useIssuesStore } from "../issues-tab/store";
import { useSessions } from "../sessions-draft";
import type { Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import type { Holder, Linked, LinkedPull, Look, Project, ProjectRow, RowState, TaskRow } from "./model";
import { allRows, rowState, useProjects, useProjectsStore } from "./store";

type Hosts = { onOpenSession: (id: string) => void; onOpenLink: (address: string) => void; onDispatch: (project: Project, rows: readonly ProjectRow[]) => void };

/** Each state's glyph and the word its tooltip names it by. An issue's open is its ticket; closed is its tracker's tick. */
function markOf(row: ProjectRow, state: RowState): { Glyph: LucideIcon; said: string } {
  if (row.kind === "issue") return state === "closed" ? { Glyph: Check, said: "Closed" } : { Glyph: Ticket, said: "Open" };
  if (state === "held") return { Glyph: CircleDot, said: "In a Job" };
  if (state === "asking") return { Glyph: MessageCircleQuestion, said: "Done?" };
  if (state === "done") return { Glyph: Check, said: "Done" };
  return { Glyph: CircleDashed, said: "Open" };
}

/** What a row is called: an issue by its reference and title. */
export const rowLabel = (row: ProjectRow): string => (row.kind === "issue" ? `${row.reference} ${row.title}` : row.title);

/** The cards a drag moves among: this project's, each holding its `[data-row]`. */
const CARDS = ".projects-item-card[data-movable]";

/** With nothing open the pane is empty: an empty slot stays empty. */
export function ProjectPane({ band, project, ...hosts }: Hosts & { band: Band | undefined; project: Project | undefined }) {
  if (band === undefined || project === undefined) return <section className="deck-pane projects-pane" aria-label="Project" tabIndex={-1} />;
  return <Pane key={project.id} band={band} project={project} {...hosts} />;
}

function Pane({ band, project, onOpenSession, onOpenLink, onDispatch }: Hosts & { band: Band; project: Project }) {
  const store = useProjectsStore()!;
  const { selected, linking, linkingTask, subtasking, look } = useProjects();
  const sessions = useSessions();
  // What Dispatch sends: the rows selected, a parent with its subtasks, and a subtask on its own where its parent is not.
  const chosen = project.rows.flatMap((row): ProjectRow[] =>
    selected.includes(row.id) ? [row] : row.kind === "task" ? (row.subtasks ?? []).filter((one) => selected.includes(one.id)) : [],
  );
  const { dragging, grab, swallowed } = useReorder((key, target, after) => store.place(key, target, after), CARDS);
  const item = (row: ProjectRow, at: number, list: string, parent?: TaskRow) => (
    <li
      key={row.id}
      className="projects-item-card"
      data-movable={list}
      data-lifted={dragging?.key === row.id || undefined}
      data-drop={dragging?.drop?.key === row.id ? (dragging.drop.after ? "after" : "before") : undefined}
    >
      <Card
        row={row}
        index={at + 1}
        look={look}
        sub={parent !== undefined}
        state={rowState(row, sessions)}
        sessions={sessions}
        selected={selected.includes(row.id) || (parent !== undefined && selected.includes(parent.id))}
        onOpenSession={onOpenSession}
        onOpenLink={onOpenLink}
        onGrab={(event) => grab(event, row.id, list)}
        swallowed={swallowed}
      />
      {linking && linkingTask === row.id && row.kind === "task" ? <LinkSearch band={band} project={project} task={row} /> : null}
      {parent === undefined && row.kind === "task" && ((row.subtasks ?? []).length > 0 || subtasking === row.id) ? (
        <ul className="projects-subtasks" aria-label={`Subtasks of ${row.title}`}>
          {(row.subtasks ?? []).map((one, n) => item(one, n, row.id, row))}
          {subtasking === row.id ? (
            <li className="projects-subtasks__add">
              <AddTask project={project} parent={row} />
            </li>
          ) : null}
        </ul>
      ) : null}
    </li>
  );
  return (
    <section className="deck-pane projects-pane" aria-label={`Project ${project.name}`} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{project.name}</h2>
        <RepositoryChip name={band.label} />
        <span className="deck-pane__acts">
          <Tooltip label="Go to the project" shortcut={PROJECTS_KEYS.rows}>
            <Kbd>{PROJECTS_KEYS.rows}</Kbd>
          </Tooltip>
          <Tooltip label="Link an issue, a Job or a pull request" shortcut={PROJECTS_KEYS.link}>
            <Button variant="ghost" size="sm" data-link-project onClick={() => store.linking(!(linking && linkingTask === undefined))} aria-pressed={linking && linkingTask === undefined}>
              Link
            </Button>
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
        {linking && linkingTask === undefined ? <LinkSearch band={band} project={project} /> : null}
      </div>
      <ul className="projects-rows" aria-label="Rows" data-look={look} data-reordering={dragging === undefined ? undefined : ""}>
        {project.rows.map((row, at) => item(row, at, project.id))}
      </ul>
    </section>
  );
}

type CardProps = {
  row: ProjectRow;
  index: number;
  look: Look;
  /** A subtask: its parent's select takes it, and it holds no subtasks or issues of its own. */
  sub: boolean;
  state: RowState;
  sessions: readonly Session[];
  selected: boolean;
  onOpenSession: (id: string) => void;
  onOpenLink: (address: string) => void;
  onGrab: (event: React.PointerEvent<HTMLElement>) => void;
  swallowed: () => boolean;
};

function Card({ row, index, look, sub, state, sessions, selected, onOpenSession, onOpenLink, onGrab, swallowed }: CardProps) {
  const store = useProjectsStore()!;
  const { Glyph, said } = markOf(row, state);
  const label = rowLabel(row);
  const interactive = (target: EventTarget) => target instanceof Element && target.closest("button, a, textarea, input") !== null;
  return (
    <div
      className="projects-row"
      data-row={row.id}
      data-state={state}
      data-look={look}
      data-sub={sub || undefined}
      data-asking={state === "asking" || undefined}
      data-selected={selected || undefined}
      tabIndex={0}
      aria-label={label}
      onPointerDown={(event) => !interactive(event.target) && onGrab(event)}
      onClickCapture={(event) => swallowed() && (event.preventDefault(), event.stopPropagation())}
    >
      <span className="projects-row__select">
        <Tooltip label="Select" shortcut={PROJECTS_KEYS.select}>
          <SelectMark tabIndex={-1} look={look} size={sub ? "sm" : undefined} index={index} checked={selected} label={`Select ${label}`} onToggle={() => store.select(row.id)} />
        </Tooltip>
      </span>
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
        <span className="projects-row__words">{row.title}</span>
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
            <Tooltip label="Link an issue, a Job or a pull request" shortcut={PROJECTS_KEYS.linkTask} asChild>
              <button type="button" tabIndex={-1} className="projects-row__act" data-link-task aria-label={`Link to ${row.title}`} onClick={() => store.linking(true, row.id)}>
                <Link size={16} strokeWidth={2} aria-hidden />
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
                  <PullRequestChip framed number={one.number} title={one.title} state={one.state} tabIndex={-1} onOpen={() => onOpenLink(one.url)} />
                  <Unlink said={`Unlink #${one.number}`} onUnlink={() => store.unlinkPull(row.id, one.number)} />
                </span>
              ))
            : null}
        </span>
      )}
    </div>
  );
}

/** A chip's own small x. */
function Unlink({ said, onUnlink }: { said: string; onUnlink: () => void }) {
  return (
    <Tooltip label="Unlink" asChild>
      <button type="button" tabIndex={-1} className="projects-row__unlink" aria-label={said} onClick={onUnlink}>
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
        <Tooltip key={one.reference} label={one.title} asChild>
          <span className="projects-row__issue" data-state={one.state} role="img" aria-label={`${one.reference} ${one.title}`}>
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
    <button type="button" tabIndex={-1} className="projects-row__job" data-job aria-label={`Open ${title}`} onClick={() => onOpen(job.id)}>
      {session === undefined ? <SessionMark state="ended" said={`${job.title}: Ended`} /> : <JobMark session={session} />}
      <span className="projects-row__job-title">{title}</span>
    </button>
  );
}

/**
 * A task, written under the project's head; Enter adds it at the top of the list and the field stays for the next. Tab
 * indents the field under the task at the top, the one just written, so Enter adds a subtask; ⇧Tab takes it back out.
 * Under a task (`parent`) the field adds its subtasks and does not indent further. A draft is kept across leaving the panel.
 */
function AddTask({ project, parent }: { project: Project; parent?: TaskRow }) {
  const store = useProjectsStore()!;
  const keep = parent === undefined ? `projects:task:${project.id}` : `projects:subtask:${parent.id}`;
  const [title, setTitle] = useState("");
  const [indented, setIndented] = useState(false);
  const top = project.rows[0]?.kind === "task" ? project.rows[0] : undefined;
  const under = parent ?? (indented ? top : undefined);
  const add = () => {
    if (title.trim() === "") return;
    if (under === undefined) store.addTask(project.id, title.trim());
    else store.addSubtask(under.id, title.trim());
    forgetKept(keep);
    setTitle("");
  };
  const said = under === undefined ? "Add a task" : `Add a subtask to ${under.title}`;
  return (
    <form
      className="projects-add"
      aria-label={said}
      data-sub={parent === undefined ? undefined : ""}
      data-indented={parent === undefined && under !== undefined ? "" : undefined}
      onSubmit={(event) => {
        event.preventDefault();
        add();
      }}
    >
      {parent === undefined && under !== undefined ? (
        <Tooltip label={`Under ${under.title}`} asChild>
          <span className="projects-add__under" role="img" aria-label={`Under ${under.title}`}>
            <ListTree size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      ) : null}
      <Textarea
        rows={1}
        keep={keep}
        autoFocus={parent !== undefined}
        aria-label={said}
        placeholder={under === undefined ? "Add a task" : "Add a subtask"}
        value={title}
        onChange={(event) => setTitle(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            add();
          }
          if (event.key === "Tab" && parent === undefined && !event.altKey && !event.metaKey && !event.ctrlKey) {
            if (event.shiftKey ? !indented : indented || top === undefined) return;
            event.preventDefault();
            setIndented(!event.shiftKey);
          }
        }}
      />
      {parent === undefined ? (
        <Tooltip label="Add a task" shortcut={PROJECTS_KEYS.addTask}>
          <Kbd>{PROJECTS_KEYS.addTask}</Kbd>
        </Tooltip>
      ) : null}
    </form>
  );
}

type Found =
  | { kind: "issue"; key: string; said: string; issue: Linked }
  | { kind: "job"; key: string; said: string; job: Holder }
  | { kind: "pull"; key: string; said: string; pull: LinkedPull };

const KIND: Record<Found["kind"], { Glyph: LucideIcon; said: string }> = {
  issue: { Glyph: Ticket, said: "Issue" },
  job: { Glyph: SquareTerminal, said: "Job" },
  pull: { Glyph: GitPullRequest, said: "Pull request" },
};

/**
 * One search of the repository: its open issues, read as the Issues tab reads them, its live Jobs and its pull requests,
 * narrowed by what he types, each with its kind's glyph. A press, or Enter on the field for the first one left, links it:
 * onto the task's card where a task asked, where a Job holds the task as Dispatch would and replaces one that did; or to
 * the project, an issue as a row and a Job or pull request as a task by its title. One already linked there is not offered.
 * It stays up for the next until Close, Esc or a press elsewhere on the page.
 */
function LinkSearch({ band, project, task }: { band: Band; project: Project; task?: TaskRow }) {
  const store = useProjectsStore()!;
  const issues = useIssuesStore();
  const { trackers } = useIssues();
  const sessions = useSessions();
  const keep = `projects:link:${task?.id ?? project.id}`;
  const [words, setWords] = useState("");
  const tracker = trackers?.find((one) => one.manifestId === band.manifestId);
  const query = tracker?.state === "read" ? tracker.open : undefined;
  useEffect(() => issues?.readTrackers([band.manifestId]), [issues, band.manifestId]);
  useEffect(() => {
    if (query !== undefined) issues?.run(band.manifestId, query);
  }, [issues, band.manifestId, query]);
  const run = query === undefined ? undefined : issues?.runOf(band.manifestId, query);
  // A press elsewhere on the page puts the search away, as Esc does; the press that opened it is not elsewhere. A page
  // leaving takes the search with it, so a press off the page is not counted.
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const away = (event: PointerEvent) => {
      const target = event.target as Element | null;
      if (target === null || target.closest(".projects-surface") === null || panel.current?.contains(target) || target.closest("[data-link-task], [data-link-project]") !== null) return;
      store.linking(false);
    };
    window.addEventListener("pointerdown", away, true);
    return () => window.removeEventListener("pointerdown", away, true);
  }, [store]);
  const rows = allRows(project);
  const issuesLinked = new Set(task === undefined ? project.rows.flatMap((row) => (row.kind === "issue" ? [row.reference] : [])) : (task.issues ?? []).map((one) => one.reference));
  const jobsLinked = new Set(task === undefined ? rows.flatMap((row) => (row.job === undefined ? [] : [row.job.id])) : task.job === undefined ? [] : [task.job.id]);
  const pullsLinked = new Set((task === undefined ? rows.flatMap((row) => (row.kind === "task" ? (row.pulls ?? []) : [])) : (task.pulls ?? [])).map((one) => one.number));
  const found: Found[] = [
    ...(run?.state === "read" ? run.tickets : [])
      .filter((one) => !issuesLinked.has(one.reference))
      .map((one): Found => ({ kind: "issue", key: one.reference, said: `${one.reference} ${one.title}`, issue: { reference: one.reference, title: one.title, url: one.url, state: one.state } })),
    ...store
      .pullsOf(band.manifestId)
      .filter((one) => !pullsLinked.has(one.number))
      .map((one): Found => ({ kind: "pull", key: `pull:${one.number}`, said: `#${one.number} ${one.title}`, pull: one })),
    ...sessions
      .filter((one) => one.manifestId === band.manifestId && one.dead === undefined && !jobsLinked.has(one.id))
      .map((one): Found => {
        const title = jobTitle(one);
        return { kind: "job", key: `job:${one.id}`, said: `Job ${title}`, job: { id: one.id, title } };
      }),
  ];
  const wanted = words.trim().toLowerCase();
  const hits = found.filter((one) => wanted === "" || one.said.toLowerCase().includes(wanted));
  const link = (one: Found) => {
    if (one.kind === "issue") return task === undefined ? store.link(project.id, one.issue) : store.linkToTask(task.id, one.issue);
    if (one.kind === "job") return task === undefined ? store.linkAsRow(project.id, one.job.title, { job: one.job }) : store.linkJob(task.id, one.job);
    return task === undefined ? store.linkAsRow(project.id, one.pull.title, { pull: one.pull }) : store.linkPull(task.id, one.pull);
  };
  const head = task === undefined ? "Link" : `Link to ${task.title}`;
  return (
    <div ref={panel} className="projects-link" role="group" aria-label={head}>
      <div className="projects-link__head">
        <span className="projects-link__title">{head}</span>
        <Tooltip label="Close" shortcut="Esc" asChild>
          <button type="button" className="projects-link__close" aria-label="Close the search" onClick={() => store.linking(false)}>
            <X size={16} strokeWidth={2} aria-hidden />
          </button>
        </Tooltip>
      </div>
      <Textarea
        rows={1}
        autoFocus
        keep={keep}
        aria-label="Search to link"
        placeholder="Issues, Jobs and pull requests"
        value={words}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            const first = hits[0];
            if (first === undefined) return;
            link(first);
            forgetKept(keep);
            setWords("");
          }
          if (event.key === "ArrowDown") {
            event.preventDefault();
            document.querySelector<HTMLElement>(".projects-link__hit")?.focus();
          }
        }}
      />
      {run?.state === "failed" ? (
        <p className="projects-link__said" role="alert">
          {run.said}
        </p>
      ) : null}
      <ul className="projects-link__hits" aria-label="Found">
        {hits.map((one) => {
          const { Glyph, said } = KIND[one.kind];
          return (
            <li key={one.key}>
              <button type="button" className="projects-link__hit" data-kind={one.kind} aria-label={one.said} onClick={() => link(one)}>
                <Tooltip label={said} asChild>
                  <span className="projects-link__kind" role="img" aria-label={said}>
                    <Glyph size={16} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
                {one.kind === "issue" ? <span className="projects-row__reference">{one.issue.reference}</span> : one.kind === "pull" ? <span className="projects-row__reference">#{one.pull.number}</span> : null}
                <span className="projects-row__words">{one.kind === "issue" ? one.issue.title : one.kind === "pull" ? one.pull.title : one.job.title}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
