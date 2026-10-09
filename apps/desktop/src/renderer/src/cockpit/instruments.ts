// What the viewscreen draws: every live Job and Session as one instrument, read off the same Board
// rows, workflows and Sessions the fleet board reads. An instrument that has a call on it takes the
// call's hue; nothing here is a figure Fleet does not serve. Mock only, as the Dashboard is.

import type { JobSummary, RepositorySummary } from "@armada/protocol";
import type { CallView } from "@armada/jobs/draft/calls";
import type { NowRunningView, NowView, NowWaitingView } from "@armada/jobs/draft/now";
import type { Session } from "@armada/screens/src/draft/sessions";
import { overviewListsOf } from "@armada/overview";
import { titleOf } from "@armada/screens";

import type { BridgeState } from "../../../shared/bridge";
import type { Item } from "../Dashboard";

/** How an instrument stands, which is which glyph it draws and how that glyph moves. */
export type Standing = "asking" | "running" | "queued" | "proposing" | "working" | "idle";

export type Instrument = {
  /** A Job's id, or `session:<id>` — the key a call's `owner` names. */
  key: string;
  title: string;
  /** Where it is: the Job's handle, or the Session's address. */
  where: string;
  standing: Standing;
  /** What the tooltip on its glyph says. */
  named: string;
  hue: Item["hue"];
  steps: readonly { id: string; label: string }[];
  /** The step it is on. */
  at: number;
  /** What it is doing now, in its own words. */
  line?: string | undefined;
  /** What is running on it: a Drone, a Check or a Judge. Absent where nothing is. */
  doing?: NowRunningView | undefined;
  /** Why nothing is running, where nothing is. */
  waiting?: NowWaitingView | undefined;
  /** When it began, for its age. */
  since?: string | undefined;
  job?: JobSummary | undefined;
  /** The call on it, where it has one. */
  call?: Item | undefined;
};

export function instrumentsOf(
  state: BridgeState,
  picked: RepositorySummary | null,
  nowViews: Readonly<Record<string, CallView>> | undefined,
  nows: Readonly<Record<string, NowView>> | undefined,
  calls: readonly Item[],
  sessions: readonly Session[],
): Instrument[] {
  const read = overviewListsOf(state.jobs, picked);
  const queued = new Set(read.sections.find((one) => one.id === "queued")?.jobs.map((one) => one.id));
  const live = read.sections.filter((one) => ["needs-you", "running", "queued", "other"].includes(one.id)).flatMap((one) => one.jobs);
  const callOf = (owner: string) => calls.find((one) => one.owner === owner);

  const jobs = live.map((job): Instrument => {
    const steps = (state.holds.workflows.find((one) => one.id === job.workflow_id)?.steps ?? []).map((step) => ({ id: step.step_id, label: step.label }));
    const call = callOf(job.id);
    const proposing = job.status === "proposing" && job.workflow_id === "";
    const standing: Standing = call !== undefined ? "asking" : queued.has(job.id) ? "queued" : proposing ? "proposing" : "running";
    return {
      key: job.id,
      title: titleOf(job),
      where: job.handle,
      standing,
      named: call?.kind ?? (standing === "queued" ? "Queued" : standing === "proposing" ? "Proposing" : "Running"),
      hue: call?.hue ?? (standing === "queued" ? "queued" : "running"),
      steps,
      at: Math.max(0, steps.findIndex((step) => step.id === job.current_step_id)),
      line: nows?.[job.id]?.running?.[0]?.line ?? nowViews?.[job.id]?.running?.[0]?.line,
      doing: nows?.[job.id]?.running?.[0],
      waiting: nows?.[job.id]?.waiting?.[0],
      since: job.started_at ?? job.created_at,
      job,
      call,
    };
  });

  const talking = sessions
    .filter((one) => one.dead === undefined)
    .map((session): Instrument => {
      const key = `session:${session.id}`;
      const call = callOf(key);
      const last = [...session.rows].reverse().find((row) => row.kind === "message");
      const working = session.turn.state === "working";
      const standing: Standing = call !== undefined ? "asking" : working ? "working" : "idle";
      return {
        key,
        title: session.title ?? session.address ?? session.id,
        where: session.address ?? session.id,
        standing,
        named: call?.doing ?? (working ? "Working" : "Idle"),
        hue: call?.hue ?? (working ? "running" : "queued"),
        steps: [],
        at: 0,
        line: last?.kind === "message" ? last.text : undefined,
        since: session.lastTurnAt,
        call,
      };
    });

  return [...jobs, ...talking];
}
