// A Job's run, read: the canvas Overview draws past the gate, for a surface that is not the Job page and holds
// no more of the Job than its whole. It builds what `JobDetail` builds for that canvas (the frozen proposal,
// where each node is, the Now panel's lit steps) and hands the canvas no edit, so every card reads.

import type { JobDetail as JobWhole, JobSummary, ManifestSummary, WorkflowSummary } from "@armada/protocol";

import { ApprovalCanvas } from "./ApprovalCanvas";
import { lifeOf } from "./approval-life";
import { litSteps } from "./draft/now";
import type { NowView } from "./draft/now";
import { stepLinesOf } from "./workflow-canvas";
import { waveReadingOf } from "./tab-wave";
import { proposalEditsOfWhole } from "./tab-proposal-read";
import type { DroneView } from "./draft/drone";

export type JobRunProps = {
  whole: JobWhole;
  /** The Job's Drones, for the line a live step draws. */
  drones?: readonly DroneView[];
  /** Every Job on the Board, for the wave a Job dispatched. */
  board?: readonly JobSummary[];
  /** The Now panel's rows: the steps they belong to stay lit and every other stands back. */
  now?: NowView | undefined;
  workflows: readonly WorkflowSummary[];
  manifest?: ManifestSummary | undefined;
  machineCap?: number | null;
  /** Open a Job the wave dispatched. */
  onOpenJob?: (jobId: string) => void;
  /** A step pressed. Absent opens the step's own card. */
  onOpenStep?: (stepId: string) => void;
};

export function JobRun({ whole, drones = [], board = [], now, workflows, manifest, machineCap = null, onOpenJob, onOpenStep }: JobRunProps) {
  return (
    <ApprovalCanvas
      whole={whole}
      edits={proposalEditsOfWhole(whole, machineCap)}
      lit={now === undefined ? undefined : litSteps(now)}
      life={lifeOf(whole, waveReadingOf(whole, undefined, board), stepLinesOf(whole, drones), Date.now())}
      {...(onOpenJob === undefined ? {} : { onOpenJob })}
      {...(onOpenStep === undefined ? {} : { onOpenStep })}
      workflows={workflows}
      manifest={manifest}
      branches={null}
      models={[]}
      harnesses={[]}
      machineCap={machineCap}
    />
  );
}
