// A Job beside the page its server serves: where it stands, its brief, and the acts JobDetail offers at
// the same moment — the decision at the review gate with the notes from this walk, or a redirect to a
// Drone still working. `docs/practices/capture-window.md`, *The panel beside the page*.
//
// **The acts are JobDetail's own**: `Decide` and `RedirectControl` from the Jobs surface, sent through
// `useCommands` as Bridge's window sends them. What it does not draw — a pull request's merge and its
// comments, the run, the record — is one press away, on Open in Bridge.

import { useCallback, useEffect, useState } from "react";
import { Badge, JOB_STATUS, Prose, WalkPanel, WalkPanelSection } from "@armada/components";
import { answerTo, Decide, RedirectControl, steeringOf } from "@armada/jobs";
import type { JobDetail, JobSummary } from "@armada/protocol";

import type { BridgeState } from "../../../shared/bridge";
import { useCommands } from "../commands";

const NONE = { state: "none" } as const;
const NOTHING = (): void => undefined;

/** The Job read whole, again whenever it moves, after an act on it, and when main says a note landed here. */
function useWhole(jobId: string, row: JobSummary | undefined, settled: boolean): [JobDetail | null, () => void] {
  const [whole, setWhole] = useState<JobDetail | null>(null);
  const again = useCallback(() => {
    void window.armada.walkPanel.readJob().then((read) => setWhole(read !== null && read.job.id === jobId ? read : null));
  }, [jobId]);
  const moved = row === undefined ? "" : `${row.status}|${row.current_step_id ?? ""}|${row.assigned_drone ?? ""}`;
  useEffect(again, [again, moved, settled]);
  useEffect(() => window.armada.walkPanel.onStale(again), [again]);
  return [whole, again];
}

export function WalkJob({ jobId, state, onRead }: { jobId: string; state: BridgeState; onRead: (state: BridgeState) => void }) {
  const row = state.jobs.find((one) => one.id === jobId);
  const commands = useCommands({ onOpen: NOTHING, onRead, jobs: state.jobs, onPaused: NOTHING });
  const deciding = commands.deciding === jobId;
  const [whole, again] = useWhole(jobId, row, deciding);
  const openInBridge = () => void window.armada.walkPanel.openInBridge();
  const job = row ?? whole?.job;
  if (job === undefined) {
    return (
      <WalkPanel kind="job" title={jobId} onOpenInBridge={openInBridge}>
        <p className="text-fg-muted">Reading the Job.</p>
      </WalkPanel>
    );
  }
  const stale = state.connection.state !== "connected";
  const rendering = JOB_STATUS[job.status];
  const steering = steeringOf(job, whole);
  const acting = commands.acting === jobId;
  return (
    <WalkPanel
      kind="job"
      title={job.handle}
      {...(rendering?.badgeStatus == null || rendering.icon === null
        ? {}
        : { status: <Badge status={rendering.badgeStatus} icon={rendering.icon}>{rendering.verb ?? job.status}</Badge> })}
      onOpenInBridge={openInBridge}
    >
      {whole?.facts === undefined ? null : (
        <WalkPanelSection head="Brief">
          <Prose text={whole.facts} />
        </WalkPanelSection>
      )}
      {job.status === "awaiting_review" ? (
        <WalkPanelSection head="Decision">
          <Decide
            job={job}
            // The evidence and the comments are Bridge's window's reads of its one open Job; asking
            // for them here would take them from it. The panel draws neither, and offers no merge.
            onNeedMaterial={NOTHING}
            onNeedRemarks={NOTHING}
            evidence={NONE}
            remarks={NONE}
            stale={stale}
            deciding={deciding}
            decidingAct={deciding ? (commands.decidingAct ?? undefined) : undefined}
            answered={commands.answeredOn(jobId)}
            onMerge={NOTHING}
            onApprove={(id) => void commands.decide(id, "approve")}
            onRequestChanges={(id, note, walk) => void commands.decide(id, "changes", note, walk)}
            onReject={(id) => void commands.decide(id, "reject")}
            onTakeUpRemarks={NOTHING}
            onOpenRemarkLink={NOTHING}
            walkNotes={whole?.walk_notes ?? []}
            onRemoveWalkNote={(id, noteId) => void window.armada.removeWalkNote(id, noteId).then(again)}
          />
        </WalkPanelSection>
      ) : steering.act === "redirect" ? (
        <WalkPanelSection head="Its Drone">
          <p className="text-fg-muted">{steering.says.redirect}</p>
          <RedirectControl
            jobId={job.id}
            drone="working"
            disabled={acting || stale}
            pending={acting && commands.actingAct === "redirect"}
            answer={answerTo(commands.answeredOn(jobId), "redirect")}
            onRedirect={(id, said) => void commands.redirect(id, said)}
          />
        </WalkPanelSection>
      ) : null}
    </WalkPanel>
  );
}
