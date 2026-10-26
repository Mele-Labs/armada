// The project open: its rows, tasks he wrote and issues he linked, in one list in his order. Each row is a box that selects
// it, its state's glyph, its title, and under the title the Job holding it. A task whose Job finished asks whether it is
// done, on the row, until he answers. The search that links issues and the field that adds a task sit at the foot.

import { useEffect, useState } from "react";
import { Check, CircleDashed, CircleDot, MessageCircleQuestion, Ticket, type LucideIcon } from "lucide-react";
import { Button, Checkbox, Kbd, RepositoryChip, SessionMark, Textarea, Tooltip, forgetKept } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { JobMark } from "../issues-tab/marks";
import { useIssues, useIssuesStore } from "../issues-tab/store";
import { useSessions } from "../sessions-draft";
import type { Band } from "./list";
import { PROJECTS_KEYS, capOf } from "./key-table";
import type { Holder, Project, ProjectRow, RowState } from "./model";
import { rowState, useProjects, useProjectsStore } from "./store";

type Hosts = { onOpenSession: (id: string) => void; onDispatch: (project: Project, rows: readonly ProjectRow[]) => void };

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

/** With nothing open the pane is empty: an empty slot stays empty. */
export function ProjectPane({ band, project, ...hosts }: Hosts & { band: Band | undefined; project: Project | undefined }) {
  if (band === undefined || project === undefined) return <section className="deck-pane projects-pane" aria-label="Project" tabIndex={-1} />;
  return <Pane key={project.id} band={band} project={project} {...hosts} />;
}

function Pane({ band, project, onOpenSession, onDispatch }: Hosts & { band: Band; project: Project }) {
  const store = useProjectsStore()!;
  const { selected, linking } = useProjects();
  const sessions = useSessions();
  const chosen = project.rows.filter((row) => selected.includes(row.id));
  return (
    <section className="deck-pane projects-pane" aria-label={`Project ${project.name}`} tabIndex={-1}>
      <header className="deck-band deck-pane__head">
        <h2 className="deck-pane__title">{project.name}</h2>
        <RepositoryChip name={band.label} />
        <span className="deck-pane__acts">
          <Tooltip label="Go to the project" shortcut={PROJECTS_KEYS.rows}>
            <Kbd>{PROJECTS_KEYS.rows}</Kbd>
          </Tooltip>
          <Tooltip label="Link issues" shortcut={PROJECTS_KEYS.link}>
            <Button variant="ghost" size="sm" onClick={() => store.linking(!linking)} aria-pressed={linking}>
              Link issues
            </Button>
          </Tooltip>
          <Tooltip label="Dispatch the rows selected to one Job" shortcut={PROJECTS_KEYS.dispatch}>
            <Button variant="secondary" size="sm" data-dispatch disabled={chosen.length === 0} onClick={() => onDispatch(project, chosen)}>
              Dispatch
            </Button>
          </Tooltip>
        </span>
      </header>
      <ul className="projects-rows" aria-label="Rows">
        {project.rows.map((row) => (
          <Row key={row.id} row={row} state={rowState(row, sessions)} sessions={sessions} selected={selected.includes(row.id)} onOpenSession={onOpenSession} />
        ))}
      </ul>
      <div className="projects-foot">
        {linking ? <LinkIssues band={band} project={project} /> : null}
        <AddTask project={project} />
      </div>
    </section>
  );
}

function Row({ row, state, sessions, selected, onOpenSession }: { row: ProjectRow; state: RowState; sessions: readonly Session[]; selected: boolean; onOpenSession: (id: string) => void }) {
  const store = useProjectsStore()!;
  const { Glyph, said } = markOf(row, state);
  const label = rowLabel(row);
  return (
    <li className="projects-row" data-row={row.id} data-state={state} data-selected={selected || undefined} tabIndex={0} aria-label={label}>
      <span className="projects-row__select">
        <Checkbox tabIndex={-1} aria-label={`Select ${label}`} checked={selected} onChange={() => store.select(row.id)}>
          {null}
        </Checkbox>
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
        {row.kind === "issue" ? <span className="projects-row__reference mono">{row.reference}</span> : null}
        <span className="projects-row__words">{row.title}</span>
      </span>
      {row.job === undefined ? null : (
        <span className="projects-row__under">
          <JobLine job={row.job} sessions={sessions} onOpen={onOpenSession} />
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
        </span>
      )}
    </li>
  );
}

/** The Job holding a row, as its live state's glyph and its title; a press opens it. One that has ended reads by the title it was sent with. */
function JobLine({ job, sessions, onOpen }: { job: Holder; sessions: readonly Session[]; onOpen: (id: string) => void }) {
  const session = sessions.find((one) => one.id === job.id);
  return (
    <span className="projects-row__job">
      {session === undefined ? <SessionMark state="ended" said={`${job.title}: Ended`} /> : <JobMark session={session} />}
      <button type="button" tabIndex={-1} className="projects-row__job-title" data-job onClick={() => onOpen(job.id)}>
        {session?.title ?? job.title}
      </button>
    </span>
  );
}

/** A task, written at the foot of the list. Enter adds it and the field stays for the next. Its draft is kept across leaving the panel. */
function AddTask({ project }: { project: Project }) {
  const store = useProjectsStore()!;
  const keep = `projects:task:${project.id}`;
  const [title, setTitle] = useState("");
  const add = () => {
    if (title.trim() === "") return;
    store.addTask(project.id, title.trim());
    forgetKept(keep);
    setTitle("");
  };
  return (
    <form
      className="projects-add"
      aria-label="Add a task"
      onSubmit={(event) => {
        event.preventDefault();
        add();
      }}
    >
      <Textarea
        rows={1}
        keep={keep}
        aria-label="Add a task"
        value={title}
        onChange={(event) => setTitle(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
            event.preventDefault();
            add();
          }
        }}
      />
      <Tooltip label="Add a task" shortcut={PROJECTS_KEYS.addTask}>
        <Kbd>{PROJECTS_KEYS.addTask}</Kbd>
      </Tooltip>
    </form>
  );
}

/**
 * The repository's open issues, read as the Issues tab reads them, narrowed by what he types. A press, or Enter on the
 * field for the first one left, links an issue as a row. One already linked is not offered.
 */
function LinkIssues({ band, project }: { band: Band; project: Project }) {
  const store = useProjectsStore()!;
  const issues = useIssuesStore();
  const { trackers } = useIssues();
  const keep = `projects:link:${project.id}`;
  const [words, setWords] = useState("");
  const tracker = trackers?.find((one) => one.manifestId === band.manifestId);
  const query = tracker?.state === "read" ? tracker.open : undefined;
  useEffect(() => issues?.readTrackers([band.manifestId]), [issues, band.manifestId]);
  useEffect(() => {
    if (query !== undefined) issues?.run(band.manifestId, query);
  }, [issues, band.manifestId, query]);
  const run = query === undefined ? undefined : issues?.runOf(band.manifestId, query);
  const linked = new Set(project.rows.flatMap((row) => (row.kind === "issue" ? [row.reference] : [])));
  const wanted = words.trim().toLowerCase();
  const hits = (run?.state === "read" ? run.tickets : []).filter((one) => !linked.has(one.reference) && (wanted === "" || `${one.reference} ${one.title}`.toLowerCase().includes(wanted)));
  const link = (one: (typeof hits)[number]) => store.link(project.id, { reference: one.reference, title: one.title, url: one.url, state: one.state });
  return (
    <div className="projects-link" role="group" aria-label="Link issues">
      <Textarea
        rows={1}
        autoFocus
        keep={keep}
        aria-label="Search issues"
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
      ) : (
        <ul className="projects-link__hits" aria-label="Issues">
          {hits.map((one) => (
            <li key={one.reference}>
              <button type="button" className="projects-link__hit" aria-label={`${one.reference} ${one.title}`} onClick={() => link(one)}>
                <span className="projects-row__reference mono">{one.reference}</span>
                <span className="projects-row__words">{one.title}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
