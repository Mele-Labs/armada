// The fleet board: every live Job a lane of its workflow's steps, each step named under its pip,
// and the merge line as a conveyor beneath. A lane that needs the owner carries a beacon. On
// Running a lane opens its context pane beside the board; on an idle Command Central the board is
// drawn alone. Mock only, as the Dashboard is.

import { useMemo, useState, type KeyboardEvent } from "react";
import { GitMerge, SquareTerminal } from "lucide-react";
import { SettlingMark, Tooltip } from "@armada/components";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import { overviewListsOf } from "@armada/overview";
import { useLayout } from "@armada/shell";

import type { BridgeState } from "../../shared/bridge";
import { CallPane } from "./CallPane";
import { titleOf } from "@armada/screens";
import { onListKey, useBoardKeys, useCursor, useItems, type Hosts, type Item } from "./Dashboard";
import { viewsOf } from "./merge-line";
import { useSessions } from "./sessions-draft";

type Lane = {
  key: string;
  title: string;
  steps: readonly { id: string; label: string }[];
  /** Where the lane is: the current step's index. */
  at: number;
  line?: string | undefined;
  /** The most pressing call on it, where it has one. */
  call?: Item | undefined;
  icon?: typeof SquareTerminal;
  /** The Job the lane is, where it is one. */
  job?: JobSummary;
  /** A Job the proposer is still writing, with no workflow settled: its rail is a caret. */
  settling?: boolean;
};

function lanesOf(
  jobs: readonly JobSummary[],
  workflows: BridgeState["holds"]["workflows"],
  nowViews: Readonly<Record<string, CallView>> | undefined,
  calls: readonly Item[],
  sessions: ReturnType<typeof useSessions>,
): Lane[] {
  const callOf = (owner: string) => calls.find((one) => one.owner === owner);
  const lanes: Lane[] = jobs.map((job) => {
    const steps = (workflows.find((one) => one.id === job.workflow_id)?.steps ?? []).map((step) => ({ id: step.step_id, label: step.label }));
    const at = Math.max(0, steps.findIndex((step) => step.id === job.current_step_id));
    const settling = job.status === "proposing" && job.workflow_id === "";
    return { key: job.id, job, title: titleOf(job), steps, at, line: nowViews?.[job.id]?.running?.[0]?.line, call: callOf(job.id), ...(settling ? { settling } : {}) };
  });
  for (const session of sessions) {
    if (session.dead !== undefined) continue;
    const last = [...session.rows].reverse().find((row) => row.kind === "message");
    lanes.push({
      key: `session:${session.id}`,
      title: session.title ?? session.id,
      steps: [],
      at: 0,
      line: last?.kind === "message" ? last.text : undefined,
      call: callOf(`session:${session.id}`),
      icon: SquareTerminal,
    });
  }
  return lanes;
}

/** One lane: its steps as named pips on a rail, the current one live, or a beacon where it needs the owner. */
function LaneRow({ lane, selected, onPick }: { lane: Lane; selected: boolean; onPick?: ((key: string) => void) | undefined }) {
  const Icon = lane.icon;
  const pips = lane.steps.length === 0 ? [{ id: "now", label: "Now" }] : lane.steps;
  const hue = lane.call?.hue ?? "running";
  return (
    <li
      className="armada-lane"
      data-job-id={lane.job?.id}
      data-status={lane.job?.status}
      data-hue={hue}
      data-focused={selected || undefined}
      data-pickable={onPick === undefined ? undefined : ""}
      role={onPick === undefined ? undefined : "option"}
      aria-label={onPick === undefined ? undefined : `${lane.title}, ${lane.icon === undefined ? "Job" : "Session"}`}
      aria-selected={onPick === undefined ? undefined : selected}
      onClick={onPick === undefined ? undefined : () => onPick(lane.key)}
    >
      <span className="armada-lane__title">
        {Icon === undefined ? null : <Icon size={14} aria-hidden="true" />}
        {lane.title}
      </span>
      {lane.call === undefined ? (
        <span className="armada-lane__beacon" aria-hidden="true" />
      ) : (
        <Tooltip label={lane.call.kind}>
          <span className="armada-lane__beacon" data-on aria-label={`${lane.title}: ${lane.call.kind}`} />
        </Tooltip>
      )}
      {lane.settling === true ? (
        <span className="armada-lane__rail">
          <SettlingMark field="Workflow" />
        </span>
      ) : (
      <ol className="armada-lane__rail" aria-label={`${lane.title}, steps`} style={{ ["--steps" as string]: pips.length }}>
        {pips.map((step, index) => {
          const state = index < lane.at ? "done" : index === lane.at ? (lane.call === undefined ? "live" : "beacon") : "ahead";
          return (
            <li key={step.id} className="armada-lane__step" data-pip={state}>
              <span className="armada-lane__pip" aria-hidden="true" />
              {lane.steps.length === 0 ? null : (
                <Tooltip label={step.label}>
                  <span className="armada-lane__name">{step.label}</span>
                </Tooltip>
              )}
            </li>
          );
        })}
      </ol>
      )}
      {lane.line === undefined ? null : <span className="armada-lane__line">{lane.line}</span>}
    </li>
  );
}

/** The merge line as a conveyor: each branch a block moving toward main, main's light at the end. */
function Conveyor({ state }: { state: BridgeState }) {
  const views = viewsOf(state);
  if (views.length === 0) return null;
  return (
    <>
      {views.map((view) => {
        const main = view.hub?.main;
        const red = main?.state === "red";
        return (
          <div key={view.root} className="armada-conveyor" aria-label="Merge line">
            <GitMerge size={14} aria-hidden="true" />
            <ol className="armada-conveyor__belt">
              {view.line.map((entry) => (
                <Tooltip key={entry.branch} label={`${entry.branch}, ${entry.state}`}>
                  <li className="armada-conveyor__block" data-state={entry.state} />
                </Tooltip>
              ))}
            </ol>
            <Tooltip label={red ? "Main is red" : "Main is green"}>
              <span className="armada-conveyor__main" data-red={red || undefined}>main</span>
            </Tooltip>
          </div>
        );
      })}
    </>
  );
}

export function FleetBoard({
  state,
  now,
  picked,
  nowViews,
  pane,
  ...hosts
}: Hosts & {
  state: BridgeState;
  now: number;
  picked: RepositorySummary | null;
  nowViews?: Readonly<Record<string, CallView>> | undefined;
  /** Whether a lane opens its context beside the board. */
  pane: boolean;
}) {
  const [answered, setAnswered] = useState<ReadonlySet<string>>(new Set());
  const calls = useItems("command-central", state, picked, nowViews, hosts, answered);
  const running = useItems("running", state, picked, nowViews, hosts, answered);
  const sessions = useSessions();
  const board = useLayout("dashboard.panels").shown.filter((one) => one.slot === "board");
  const [selected, setSelected] = useState<string>();

  const lanes = useMemo(() => {
    const read = overviewListsOf(state.jobs, picked);
    const live = read.sections.filter((one) => ["needs-you", "running", "queued", "other"].includes(one.id)).flatMap((one) => one.jobs);
    // A status this build's registry does not know has no badge to draw: its lane names the status instead.
    const unknown = read.undrawable.map((job): Lane => ({ key: job.id, job, title: titleOf(job), steps: [], at: 0, line: job.status }));
    return [...lanesOf(live, state.holds.workflows, nowViews, calls, sessions), ...unknown];
  }, [state, picked, nowViews, calls, sessions]);

  const current = lanes.find((one) => one.key === selected) ?? lanes[0];
  const item = current === undefined ? undefined : (current.call ?? running.find((one) => one.key === current.key || one.owner === current.key));

  useCursor(pane ? current?.job : undefined, hosts.onCursor);
  const at = lanes.findIndex((one) => one.key === current?.key);
  useBoardKeys(lanes, at, setSelected, hosts, pane);
  const move = (event: KeyboardEvent) => onListKey(event, lanes, at, setSelected);

  return (
    <div className="armada-board" data-pane={pane || undefined}>
      <section className="armada-deck__fleet" aria-label="Fleet">
        {/* The panels a layout orders and hides: Fleet, then the merge line, until one says otherwise. */}
        {board.map((one) =>
          one.id === "merge-line" ? (
            <Conveyor key={one.id} state={state} />
          ) : (
            <ul key={one.id} className="armada-deck__lanes" role={pane ? "listbox" : undefined} aria-label={pane ? "Lanes" : undefined} tabIndex={pane ? 0 : undefined} onKeyDown={pane ? move : undefined}>
              {lanes.map((lane) => (
                <LaneRow key={lane.key} lane={lane} selected={pane && lane.key === current?.key} onPick={pane ? setSelected : undefined} />
              ))}
            </ul>
          ),
        )}
      </section>
      {pane && item !== undefined ? (
        <CallPane item={item} now={now} workflows={state.holds.workflows} onDone={() => setAnswered(new Set([...answered, item.key]))} onOpenSession={hosts.onOpenSession} onOpenJob={hosts.onOpen} onOpenLink={hosts.onOpenLink} />
      ) : null}
    </div>
  );
}
