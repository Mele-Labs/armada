// Jobs, on the rail, in the Sessions page's own layout: one frame with a header band, the tabs under it
// (Active, Done, All), and every heading a column of tiles on a ruled glass. A Job's tile is a Session's:
// the title on top with its state's mark, the time at the top right, and the facts below on one line that
// never wraps. State is the tile's edge and the mark, and only a Job that is running turns it. A running
// Job also carries a one-line Now: what its Drone is doing at this moment, and how long ago.
// Beside its pull request a live Job draws its plan's tasks as a bar, done first; the figure is the tooltip's.
// The sections, the fold of a lineage and the order are Overview's own (`overviewListsOf`); the acts are
// the Board row's, the less used behind the ellipsis. j and k, Enter, x and n are the Board's keys.

import { useMemo, useState } from "react";
import { Ellipsis, LoaderCircle } from "lucide-react";
import { Button, Chip, DropdownMenu, StepBar, Tabs, Tooltip, useNow, usePilot, useOpenInSession } from "@armada/components";
import type { DropdownMenuEntry } from "@armada/components";
import type { JobSummary, MergeLines, RepositorySummary } from "@armada/protocol";
import type { NowView } from "@armada/jobs/draft/now";
import { JobMarks, hasMarks, isTerminal, titleOf as plainTitle, repositoryOf, sectionOf, readingOf, canPause, canResume, elapsedOf, absoluteOf, taskBarSegmentsOf, taskFigureOf, tasksOwed, ACT_LABEL } from "@armada/screens";
import type { BoardSection, PauseAct } from "@armada/screens";
import { BoardEmpty, OverviewEmpty } from "@armada/screens/src/BoardEmpty";
import { boardPressOf } from "@armada/screens/src/keys";
import { useListCursor, useListKeydown } from "@armada/screens/src/list-keyboard";
import { headlineOf, overviewListsOf } from "@armada/overview";
import { Boundary } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import "./jobs-surface.css";

const VIEWS = [
  { id: "active", label: "Active", sections: ["needs-you", "running", "queued", "other"] },
  { id: "done", label: "Done", sections: ["recently-ended", "done"] },
  { id: "all", label: "All", sections: ["needs-you", "running", "queued", "other", "recently-ended", "done"] },
] as const satisfies readonly { id: string; label: string; sections: readonly BoardSection[] }[];

type ViewId = (typeof VIEWS)[number]["id"];

/** A status that needs him or has failed: the tile is lit, as a Session's is. */
const LIT = new Set(["awaiting-review", "escalated", "completed-failed"]);

/** Time since `at`, as the Sessions list says it: `4m`, `2h`, `3d`. */
function agoOf(at: string, now: number): string | undefined {
  const then = Date.parse(at);
  if (Number.isNaN(then)) return undefined;
  const minutes = Math.max(0, Math.floor((now - then) / 60_000));
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours}h` : `${Math.floor(hours / 24)}d`;
}

/** Seconds, then minutes: how long ago a Drone did what its Now line says. */
function sinceOf(at: string, now: number): string {
  const seconds = Math.max(0, Math.floor((now - Date.parse(at)) / 1000));
  return seconds < 60 ? `${seconds}s` : seconds < 3600 ? `${Math.floor(seconds / 60)}m` : `${Math.floor(seconds / 3600)}h`;
}

/** The pull request on this Job's branch, as the forge lists it. */
function pullOf(job: JobSummary, lines: MergeLines | null) {
  if (job.branch === undefined) return undefined;
  for (const line of lines?.lines ?? []) {
    const found = line.hub?.pull_requests?.find((one) => one.branch === job.branch);
    if (found !== undefined) return found;
  }
  return undefined;
}

type Handlers = {
  onOpen: (jobId: string) => void;
  onKill: (jobId: string) => void;
  onRedispatch: (jobId: string) => void;
  onClear: (jobId: string) => void;
  onPausing: (act: PauseAct, jobId: string) => void;
};

/** What a running Job's Drone is doing now: its step, its latest line, and when. Mock only: Fleet serves no `NowView`. */
function nowOf(job: JobSummary, nows: Readonly<Record<string, NowView>> | undefined) {
  if (job.status !== "running") return undefined;
  const drone = nows?.[job.id]?.running?.find((one) => one.of === "drone");
  if (drone === undefined) return undefined;
  const doing = drone.tail?.[drone.tail.length - 1] ?? drone.line;
  const step = drone.step?.name ?? job.current_step_id;
  return doing === undefined || drone.since === undefined ? undefined : { step, doing, since: drone.since };
}

// The three figures a tile draws off the clock. Each reads it here, so a tick draws these and not the rows.

/** The time at the tile's top right: the run time while it runs, how long ago it started or ended otherwise. */
function Last({ job, running }: { job: JobSummary; running: boolean }) {
  const now = useNow();
  const time = running ? elapsedOf(job, now) : agoOf(job.ended_at ?? job.created_at, now);
  if (time === undefined) return null;
  return (
    <Tooltip label={running ? "Run time" : job.ended_at === undefined ? `Dispatched ${absoluteOf(job.created_at) ?? ""}` : `Ended ${absoluteOf(job.ended_at) ?? ""}`}>
      <span className="armada-jobs-page__last">{time}</span>
    </Tooltip>
  );
}

function Marks({ job }: { job: JobSummary }) {
  const now = useNow();
  return hasMarks(job, now) ? <JobMarks job={job} now={now} /> : null;
}

/** A running Job's Now line: what its Drone is doing, and how long ago. */
function DoingNow({ doing, since }: { doing: string; since: string }) {
  const age = sinceOf(since, useNow());
  return (
    <Tooltip label={`${doing} · ${age}`} asChild>
      <div className="armada-jobs-page__now">
        <LoaderCircle size={12} strokeWidth={2} aria-hidden />
        <span className="armada-jobs-page__now-doing">{doing}</span>
        <span className="armada-jobs-page__now-age">· {age}</span>
      </div>
    </Tooltip>
  );
}

function Tile({
  job,
  headline,
  repository,
  lines,
  nows,
  ...acts
}: {
  job: JobSummary;
  headline: string;
  repository: string | undefined;
  lines: MergeLines | null;
  nows: Readonly<Record<string, NowView>> | undefined;
} & Handlers) {
  const reading = readingOf(job);
  const pilot = usePilot();
  const session = useOpenInSession();
  if (reading.as !== "badge") return null;
  const Mark = reading.icon;
  const running = job.status === "running";
  const ended = isTerminal(job);
  const pull = pullOf(job, lines);
  const doing = nowOf(job, nows);
  const step = doing?.step ?? job.current_step_id;
  const recent = sectionOf(job) === "recently-ended";
  const pausing: PauseAct | null = canResume(job) ? "resume_job" : canPause(job) ? "pause_job" : null;
  const entries: DropdownMenuEntry[] = [
    ...(recent && job.status !== "rejected" ? [{ kind: "item" as const, id: "redispatch", label: ACT_LABEL.redispatch }] : []),
    ...(pilot !== null && running ? [{ kind: "item" as const, id: "pilot", label: "Pilot" }] : []),
    ...(session !== null && (job.status === "escalated" || job.status === "completed_failed") ? [{ kind: "item" as const, id: "session", label: "Open in a Session" }] : []),
    ...(pausing === null ? [] : [{ kind: "item" as const, id: pausing, label: ACT_LABEL[pausing] }]),
    ...(recent ? [{ kind: "item" as const, id: "clear", label: "Clear" }] : []),
    ...(ended ? [] : [{ kind: "separator" as const, id: "rule" }, { kind: "item" as const, id: "kill", label: "Kill", shortcut: "x", danger: true }]),
  ];
  const choose = (id: string) => {
    if (id === "redispatch") acts.onRedispatch(job.id);
    else if (id === "pilot") pilot?.ask(job.id);
    else if (id === "session") session?.open(job.id);
    else if (id === "pause_job" || id === "resume_job") acts.onPausing(id, job.id);
    else if (id === "clear") acts.onClear(job.id);
    else if (id === "kill") acts.onKill(job.id);
  };
  const checks = pull?.ci === undefined ? undefined : pull.ci === "passed" ? { state: "passed" as const, said: "Checks passed" } : pull.ci === "failed" ? { state: "failed" as const, said: "Checks failed" } : { state: "pending" as const, said: "Checks running" };
  const facts = [
    step === undefined ? null : (
      <Tooltip key="step" label="Step">
        <span>{step}</span>
      </Tooltip>
    ),
    pull === undefined ? null : <Chip key="pr" chip={{ kind: "pull_request", number: pull.number }} {...(checks === undefined ? {} : { checks })} />,
    // How far along it is in its step: the plan's tasks, as `JobSummary` counts them. The branch is on the Job's detail.
    ended || job.tasks === undefined || tasksOwed(job.tasks) === 0 ? null : (
      <StepBar key="tasks" tasks={taskBarSegmentsOf(job.tasks)} label={`${doing?.step ?? job.current_step_id ?? "Plan"} · ${taskFigureOf(job.tasks)} tasks done`} />
    ),
    repository === undefined ? null : (
      <Tooltip key="repository" label="Repository">
        <span>{repository}</span>
      </Tooltip>
    ),
  ].filter((one) => one !== null);
  return (
    <li className="armada-jobs-page__row" data-status={reading.status} data-lit={LIT.has(reading.status) || undefined} data-ended={ended || undefined} style={{ ["--hue" as string]: `var(--status-${reading.status})` }}>
      <div className="armada-jobs-page__top">
        <button type="button" className="armada-jobs-page__open" data-job-id={job.id} aria-label={headline} onClick={() => acts.onOpen(job.id)}>
          <Tooltip label={reading.verb}>
            <span className="armada-jobs-page__mark" role="img" aria-label={reading.verb} data-pulsing={(running && job.paused === undefined) || undefined}>
              <Mark size={16} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
          <span className="armada-jobs-page__title">{headline}</span>
          <Marks job={job} />
        </button>
        <Last job={job} running={running} />
        {entries.length === 0 ? null : (
          <span className="armada-jobs-page__more">
            <DropdownMenu triggerLabel={`More for ${plainTitle(job)}`} icon={Ellipsis} entries={entries} onSelect={choose} portal />
          </span>
        )}
      </div>
      {doing === undefined ? null : (
        <DoingNow doing={doing.doing} since={doing.since} />
      )}
      {facts.length === 0 ? null : <div className="armada-jobs-page__facts">{facts}</div>}
    </li>
  );
}

export function JobsSurface({
  state,
  repositories,
  disconnected,
  nows,
  onOpen,
  onKill,
  onRedispatch,
  onClear,
  onPausing,
  onCompose,
  onCopied,
  onCursor,
}: {
  state: BridgeState;
  repositories: readonly RepositorySummary[];
  disconnected: string | null;
  /** What each running Job's Drone is doing now, by Job id. Mock only. */
  nows?: Readonly<Record<string, NowView>> | undefined;
  onCompose: () => void;
  onCopied: (value: string) => void;
  onCursor: (jobId: string | null) => void;
} & Handlers) {
  const [view, setView] = useState<ViewId>("active");
  const picked = repositories.find((one) => one.root === state.repository) ?? null;
  const read = useMemo(() => overviewListsOf(state.jobs, picked), [state.jobs, picked]);
  const wanted = new Set<string>(VIEWS.find((one) => one.id === view)!.sections);
  const groups = read.sections.filter((one) => wanted.has(one.id));
  const all = state.repository === null;
  // Fleet unreachable, still starting, or nothing served: the Board's own words, whatever the tab.
  const fault = disconnected !== null || state.connection.state === "starting" || repositories.length === 0;

  const { cursor, onFocusCapture, move } = useListCursor(onCursor);
  const drawn = groups.flatMap((one) => one.jobs);
  useListKeydown((event) => {
    const press = boardPressOf(event);
    if (press === null) return;
    const job = drawn.find((one) => one.id === cursor);
    if (press.act === "move") move(press.by);
    else if (press.act === "open" && job !== undefined) onOpen(job.id);
    else if (press.act === "kill" && job !== undefined && !isTerminal(job)) onKill(job.id);
    else if (press.act === "compose") onCompose();
    else return;
    event.preventDefault();
  });

  return (
    <Boundary region="Jobs" bridge={state.bridge} onCopied={onCopied}>
      <div className="armada-screen__overview">
        <section className="armada-jobs-page" aria-label="Jobs">
          <div className="armada-jobs-page__head">
            <h2 className="armada-jobs-page__title-bar">Jobs</h2>
            <Button size="sm" variant="secondary" onClick={onCompose}>
              New Job
            </Button>
          </div>
          <div className="armada-jobs-page__controls">
            <Tabs items={VIEWS.map(({ id, label }) => ({ id, label }))} value={view} onChange={(id) => setView(id as ViewId)} />
          </div>
          <div className="armada-jobs-page__glass" onFocusCapture={onFocusCapture}>
            {fault ? (
              <BoardEmpty disconnected={disconnected} starting={state.connection.state === "starting"} why={null} suspended={false} nothingServed={repositories.length === 0} onClear={() => {}} />
            ) : state.jobs.length === 0 ? (
              <OverviewEmpty onCompose={onCompose} />
            ) : (
              groups.map((group) => (
                <div className="armada-jobs-page__group" key={group.id}>
                  <h3 className="armada-jobs-page__eyebrow">{group.label}</h3>
                  <ul className="armada-jobs-page__rows" aria-label={group.label}>
                    {group.jobs.map((job) => (
                      <Tile
                        key={job.id}
                        job={job}
                        headline={headlineOf(job, read.dispatch.get(job.id))}
                        repository={repositoryOf(job, repositories, all)}
                        lines={state.mergeLines}
                        nows={nows}
                        onOpen={onOpen}
                        onKill={onKill}
                        onRedispatch={onRedispatch}
                        onClear={onClear}
                        onPausing={onPausing}
                      />
                    ))}
                  </ul>
                </div>
              ))
            )}
          </div>
        </section>
      </div>
    </Boundary>
  );
}
