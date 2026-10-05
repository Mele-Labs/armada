// Overview's lists: Needs you, Running, Queued and Other, over the Jobs in scope. #920.
//
// **Read from `sectionsOf`, never restated.** `overviewListsOf` in `overview-lists.ts` scopes the
// board and applies the Board's own fold and sort ahead of it.
//
// **Rows are the Board's own `Row`** — same field run, same act, and on All with more than one
// repository served the row names it, as the Board's own list did.
//
// **Each section is its own panel** — `ActiveJobsList`'s `panel` variant.
// Overview 27 (#1091) gave the variant its own fold; `openSections` and `onSectionOpenChange` are
// this screen's own pass-through, and the caller is who remembers a press across a restart.
//
// **Disconnected reads exactly as `BoardEmpty`'s own fault state.** Overview holds whatever Bridge
// last received, so a Needs you row from before an outage stays on screen; only a board with
// nothing on it at all draws the disconnected message.
//
// **The keyboard is `list-keyboard.ts`'s mechanism and `keys.ts`'s map** — the window listener,
// DOM-focus-as-cursor, and `boardPressOf`. `move`, `open`, `verb`, `kill` and `compose` are
// answered; `copy` reaches nothing on a healthy Job. `search` and the state tabs went with the Job
// Board, which held the only field and the only tab strip.
//
// Mounted beneath the summary strip #921 put here. Since the Job Board went, this is where every
// Job is listed, Done included — `overview-lists.ts` says why that section arrived.

import { ActiveJobsList, Button, Tooltip, WaveCanvas } from "@armada/components";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import { ChartGantt, Waypoints } from "lucide-react";
import { BoardEmpty, OverviewEmpty } from "./BoardEmpty";
import type { BoardSection } from "./board";
import { columnsFor, repositoryOf } from "./board";
import { boardPressOf, verbOf } from "./keys";
import { headlineOf } from "./lineage";
import { useListCursor, useListKeydown } from "./list-keyboard";
import { overviewGraphOf } from "./overview-graph";
import { nestedOf, overviewListsOf } from "./overview-lists";
import { OverviewTimeline } from "./OverviewTimeline";
import { readingOf } from "./reading";
import { useRecentChanges } from "./recent";
import { isTerminal, Row } from "./Row";

/** `id` on a section's own outer element, so a press elsewhere can `scrollIntoView` it by name. */
export function overviewPanelId(section: BoardSection): string {
  return `armada-overview-panel-${section}`;
}

export type OverviewListsProps = {
  jobs: readonly JobSummary[];
  /** True while what is held is not live — every row reads de-emphasised, the Board's own rule. */
  stale: boolean;
  now: number;
  workflows: readonly WorkflowSummary[];
  /** Every repository Fleet serves. */
  repositories: readonly RepositorySummary[];
  /** The rail's pick, by root. `null` is All repositories. */
  picked: string | null;
  /** The connection's own statement, where Fleet cannot be reached — `BoardEmpty`'s fault state. */
  disconnected: string | null;
  /** The Job whose detail is open, where one is. */
  selected: string | null;
  /**
   * Whether each section's panel is open, keyed by `BoardSection`. Absent for
   * a section reads open — `ActiveJobsList`'s own default, `Panel`'s rule.
   * Overview 24's mechanism remembers it; this only reads what it is told.
   */
  openSections?: Partial<Record<BoardSection, boolean>>;
  /** A panel's fold pressed. Absent for a section leaves it always open. */
  onSectionOpenChange?: (section: BoardSection, open: boolean) => void;
  /** Open a Job. Every row is a control, so every row calls this. */
  onOpen: (jobId: string) => void;
  /** Ask to kill the Job a row's own control names. It asks; it never kills — `Jobs.tsx`'s rule. */
  onKill: (jobId: string) => void;
  /** Ask to redispatch — Recently ended's own control, Overview 28 (#1092). Asks; never redispatches. */
  onRedispatch: (jobId: string) => void;
  /** Ask to clear — Recently ended's caret, beside Redispatch. Asks; never clears. */
  onClear: (jobId: string) => void;
  /**
   * Open the composer — `n`, the one key in the contextual tier that acts on
   * nothing on screen. `Jobs.tsx`'s own prop, answered here too: the map
   * already recognized the key, and this is what it now reaches.
   */
  onCompose: () => void;
  /** A clipboard write is silent, so the surface confirms every one with a toast. */
  onCopied: (value: string) => void;
  /**
   * Where the cursor is, reported up. **This holds it, `Jobs.tsx`'s own
   * pattern** — DOM focus is the cursor, whichever panel's row holds it, so
   * `j`, the arrows, Tab and the mouse all move one cursor across every
   * section. When focus leaves every panel the last row stands, the same as
   * the Board.
   */
  onCursor?: (jobId: string | null) => void;
  /** The Jobs as the sectioned list or as the graph their dependencies make. Absent is the list. */
  view?: OverviewView;
  /** The toggle pressed. Absent draws no toggle. */
  onView?: (view: OverviewView) => void;
  /** Where the timeline's playhead is, in epoch milliseconds. Null is now. */
  playhead?: number | null;
  onPlayhead?: (t: number | null) => void;
};

export type OverviewView = "list" | "graph" | "timeline";

export function OverviewLists({
  jobs,
  stale,
  now,
  workflows,
  repositories,
  picked,
  disconnected,
  selected,
  openSections,
  onSectionOpenChange,
  onOpen,
  onKill,
  onRedispatch,
  onClear,
  onCompose,
  onCopied,
  onCursor,
  view = "list",
  onView,
  playhead = null,
  onPlayhead,
}: OverviewListsProps) {
  const pickedRepository = repositories.find((one) => one.root === picked) ?? null;
  const all = picked === null;
  const { sections, dispatch, undrawable } = overviewListsOf(jobs, pickedRepository);
  // The Board's own call, `Jobs.tsx`'s own name for it: the columns a row's own fact set names,
  // shared down the list so a column lines up whether or not every row carries that fact.
  const columns = columnsFor(jobs, repositories, all);
  // The cursor, and the keys that move or act on it — `list-keyboard.ts`'s shared mechanism.
  const { cursor, onFocusCapture, move } = useListCursor(onCursor);
  // Which rows changed while this was showing them — `recent.ts`, the Board's own mark.
  const recent = useRecentChanges(jobs);
  // Every drawn row, flattened across sections — what a verb or a kill key checks the cursor against.
  const drawn = sections.flatMap((section) => section.jobs);

  function press(event: KeyboardEvent): void {
    const read = boardPressOf(event);
    if (read === null) return;
    const job = drawn.find((one) => one.id === cursor);
    switch (read.act) {
      case "move":
        move(read.by);
        break;
      case "open":
        if (job === undefined) return;
        onOpen(job.id);
        break;
      case "verb":
        // The row carries one control, so at most one verb key applies — `Jobs.tsx`'s own rule.
        if (job === undefined) return;
        if (verbOf(job, isTerminal(job)) !== read.verb) return;
        onOpen(job.id);
        break;
      case "kill":
        if (job === undefined || isTerminal(job)) return;
        onKill(job.id);
        break;
      case "compose":
        onCompose();
        break;
      case "copy":
        // Nothing to copy on a healthy Job — `copyDebugInfoFor` takes a
        // `Failure`. The map still claims the key so it is where the contract
        // says it is; the press does nothing and is not swallowed.
        return;
    }
    event.preventDefault();
  }
  useListKeydown(press);

  const graph = view === "graph" ? overviewGraphOf(drawn, { onOpen, onKill, onRedispatch }) : null;

  const rowOf = ({ job, depth, alsoWaits }: { job: JobSummary; depth: number; alsoWaits: boolean }) => (
    <Row
      key={job.id}
      job={job}
      depth={depth}
      alsoWaits={alsoWaits}
      headline={headlineOf(job, dispatch.get(job.id))}
      stale={stale}
      now={now}
      workflows={workflows}
      repository={repositoryOf(job, repositories, all)}
      selected={job.id === selected}
      focused={job.id === cursor}
      recent={recent.get(job.id)}
      onOpen={onOpen}
      onKill={onKill}
      onRedispatch={onRedispatch}
      onClear={onClear}
      onCopied={onCopied}
    />
  );

  return (
    <div
      className="armada-screen__overview-lists"
      // The cursor is DOM focus, across every panel — a row reached by the
      // mouse, by Tab or by a panel's own arrows all set the same value,
      // whichever section it is in.
      onFocusCapture={onFocusCapture}
    >
      {onView === undefined || sections.length === 0 ? null : (
        <div className="armada-screen__overview-view">
          {/* `waypoints` and `chart-gantt`, the registry's graph and timeline toggles: pressed is that view. */}
          <Tooltip label={view === "graph" ? "Back to the list" : "Graph"}>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Graph"
              aria-pressed={view === "graph"}
              onClick={() => onView(view === "graph" ? "list" : "graph")}
            >
              <Waypoints size={16} />
            </Button>
          </Tooltip>
          <Tooltip label={view === "timeline" ? "Back to the list" : "Timeline"}>
            <Button
              variant="ghost"
              size="sm"
              iconOnly
              aria-label="Timeline"
              aria-pressed={view === "timeline"}
              onClick={() => onView(view === "timeline" ? "list" : "timeline")}
            >
              <ChartGantt size={16} />
            </Button>
          </Tooltip>
        </div>
      )}
      {view === "timeline" && sections.length > 0 ? (
        <OverviewTimeline
          jobs={drawn}
          now={now}
          playhead={playhead}
          onPlayhead={(t) => onPlayhead?.(t)}
          acts={{ onOpen, onKill, onRedispatch }}
        />
      ) : graph !== null && sections.length > 0 ? (
        <div className="armada-screen__overview-graph">
          <WaveCanvas nodes={graph.nodes} edges={graph.edges} label="Jobs and what they wait on" />
        </div>
      ) : sections.length === 0 && disconnected === null && repositories.length > 0 ? (
        // The null result is a card of its own on the canvas, not a well inside a panel — a
        // card inside the panel would be a card in a card. `BoardEmpty`'s order: a fault or a
        // fresh install reads first, and both keep the panel below unchanged. #1262.
        //
        // The wrapper is the empty panel's own frame, kept without the panel: `ActiveJobsList`
        // named its empty frame by `label`; a region keeps that name without a list of no items, so
        // assistive tech still finds "Overview" here. No class, so it draws nothing of its own.
        <div role="region" aria-label="Overview">
          <OverviewEmpty onCompose={onCompose} />
        </div>
      ) : sections.length === 0 ? (
        <ActiveJobsList
          variant="panel"
          selectable
          label="Overview"
          empty={
            <BoardEmpty
              disconnected={disconnected}
              why={null}
              suspended={false}
              nothingServed={repositories.length === 0}
              onClear={() => {}}
            />
          }
        />
      ) : (
        sections.map((section) => (
          <ActiveJobsList
            key={section.id}
            id={overviewPanelId(section.id)}
            variant="panel"
            heading={section.label}
            count={section.jobs.length}
            open={openSections?.[section.id]}
            onOpenChange={
              onSectionOpenChange ? (open) => onSectionOpenChange(section.id, open) : undefined
            }
            selectable
            label={section.label}
            columns={columns}
          >
            {nestedOf(section.jobs).map(rowOf)}
          </ActiveJobsList>
        ))
      )}

      {/* The registry has no glyph for this state, so the row shape cannot draw it — named
          rather than dropped, `Jobs.tsx`'s own choice for the same case. */}
      {undrawable.map((job) => {
        const reading = readingOf(job);
        if (reading.as === "badge") return null;
        return (
          <p key={job.id} className="text-fg-muted">
            {`${headlineOf(job, dispatch.get(job.id))} — `}
            <span className="mono">{reading.wire}</span>
            {`. The registry carries no ${reading.missing.join(" and no ")} for it, so the row shape cannot draw it.`}
          </p>
        );
      })}
    </div>
  );
}
