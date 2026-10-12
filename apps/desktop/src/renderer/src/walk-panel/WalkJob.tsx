// A Job beside the page its server serves, drawn live: where it stands, its brief, and the verdict
// JobDetail draws at the same moment — the evidence, the pull request with its comments and its Merge,
// and the decision with this walk's notes — or a redirect to a Drone still working.
// `docs/practices/capture-window.md`, *The panel beside the page*.
//
// **It reads Bridge's one open Job**, which this window's Job became when the window opened or took
// focus (owner, 11 Oct 2026). Nothing here reads a Job of its own: the detail, the claims and the
// comments are the shared state every Bridge window draws, and the acts go through `useCommands` as
// Bridge's window sends them.

import { useEffect, useState } from "react";
import { Badge, JOB_STATUS, Prose, WalkPanel, WalkPanelSection } from "@armada/components";
import {
  answerTo,
  detailOf,
  FIRST_PLAN_VIEW,
  RedirectControl,
  renderFor,
  steeringOf,
  verdictSlotOf,
  type PlanView,
} from "@armada/jobs";
import { ordered } from "@armada/screens/src/facts";

import type { BridgeState } from "../../../shared/bridge";
import { useNow } from "../app-effects";
import { openArtifact, openPullRequest, openRemarkLink, readEvidence, readRemarks, useCommands } from "../commands";

const NOTHING = (): void => undefined;

/**
 * Ask for a read and never let it go. **Bridge's window holds the same read for the same Job**, and a
 * panel closing with the walk window would drop it from under that window; the next Job Bridge opens
 * asks for its own, which replaces this one.
 */
const asking = (read: (jobId: string | null) => void) => (jobId: string | null) => {
  if (jobId !== null) read(jobId);
};
const NEED_MATERIAL = asking(readEvidence);
const NEED_REMARKS = asking(readRemarks);

export function WalkJob({ jobId, state, onRead }: { jobId: string; state: BridgeState; onRead: (state: BridgeState) => void }) {
  const commands = useCommands({ onOpen: NOTHING, onRead, jobs: state.jobs, onPaused: NOTHING });
  const now = useNow();
  const [said, setSaid] = useState<string | null>(null);
  const [planView, setPlanView] = useState<PlanView>(FIRST_PLAN_VIEW);
  const whole = detailOf(state.watched, jobId);
  // Fleet's own read once it has arrived, for `JobDetail`'s reason: the board's row can lag an event.
  const job = whole?.job ?? state.jobs.find((one) => one.id === jobId);
  const render = job === undefined ? undefined : renderFor(job);
  // The claims, for as long as there is a verdict to draw — what `JobDetail`'s Overview holds them for.
  useEffect(() => {
    if (render === "reviewing" || render === "finished") NEED_MATERIAL(jobId);
  }, [jobId, render]);
  const openInBridge = () => void window.armada.walkPanel.openInBridge();
  if (job === undefined || render === undefined) {
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
  const deciding = commands.deciding === jobId;
  const steps = ordered(whole);
  const open = steps.find((step) => step.step_id === job.current_step_id) ?? steps[0];
  const claimed =
    state.evidence.state === "read" ? state.evidence.steps.find((one) => one.step_id === open?.step_id) : undefined;
  // Bridge's window opened another Job since this window last had focus. Focus brings this one back.
  const elsewhere = whole === null && state.watched.state !== "none" && state.watched.jobId !== jobId;
  const verdict =
    whole === null
      ? undefined
      : verdictSlotOf({
          job,
          whole,
          open,
          render,
          recorded: { diff: state.diff, evidence: state.evidence, remarks: state.remarks },
          opensRecords: { jobId, open: openArtifact, onSaid: setSaid },
          now,
          claimed,
          notes: [],
          onNeedMaterial: NOTHING,
          onNeedRemarks: NEED_REMARKS,
          stale,
          acting,
          actingAct: acting ? (commands.actingAct ?? undefined) : undefined,
          deciding,
          decidingAct: deciding ? (commands.decidingAct ?? undefined) : undefined,
          answered: commands.answeredOn(jobId),
          ...commands.mergeProps(jobId),
          onRerunFailedChecks: (id) => void commands.rerunFailedChecks(id),
          onInvestigateFailedChecks: (id) => void commands.investigateFailedChecks(id),
          onApproveReview: (id) => void commands.decide(id, "approve"),
          onRequestChanges: (id, note, walk) => void commands.decide(id, "changes", note, walk),
          onRemoveWalkNote: (id, noteId) => void window.armada.removeWalkNote(id, noteId),
          onReject: (id) => void commands.decide(id, "reject"),
          onTakeUpRemarks: (id, remarks) => void commands.takeUpRemarks(id, remarks),
          onDismissFinding: (id, finding, reason) => void commands.dismissFinding(id, finding, reason),
          onOpenRemarkLink: (id, remarkId) => void openRemarkLink(id, remarkId),
          onOpenPullRequest: openPullRequest,
          onSaid: setSaid,
          onAnswerJudge: (id, askedAt, answer, note) => void commands.answerJudge(id, askedAt, answer, note),
          // A plan at the gate is Plan's own review, as on Bridge's window, folded to the panel's width.
          plan: {
            job,
            whole,
            floor: true,
            view: planView,
            onView: setPlanView,
            stale,
            acting,
            deciding,
            onApproveReview: (id) => void commands.decide(id, "approve"),
            onAnswerJudge: (id, askedAt, answer, note) => void commands.answerJudge(id, askedAt, answer, note),
            onRedirect: (id, instruction, droneId) => void commands.redirect(id, instruction, droneId),
            board: state.jobs,
            onSaid: setSaid,
            diff: state.diff,
            now,
          },
        });
  return (
    <WalkPanel
      kind="job"
      title={job.handle}
      {...(rendering?.badgeStatus == null || rendering.icon === null
        ? {}
        : { status: <Badge status={rendering.badgeStatus} icon={rendering.icon}>{rendering.verb ?? job.status}</Badge> })}
      onOpenInBridge={openInBridge}
    >
      {elsewhere ? (
        <p className="text-fg-muted">Bridge has another Job open. This one comes back when this window has focus.</p>
      ) : null}
      {whole?.facts === undefined ? null : (
        <WalkPanelSection head="Brief">
          <Prose text={whole.facts} />
        </WalkPanelSection>
      )}
      {verdict === undefined || verdict === null ? null : (
        <WalkPanelSection head={render === "reviewing" ? "Decision" : "Verdict"}>{verdict}</WalkPanelSection>
      )}
      {steering.act === "redirect" ? (
        <WalkPanelSection head="Its Drone">
          <p className="text-fg-muted">{steering.says.redirect}</p>
          <RedirectControl
            jobId={job.id}
            drone="working"
            disabled={acting || stale}
            pending={acting && commands.actingAct === "redirect"}
            answer={answerTo(commands.answeredOn(jobId), "redirect")}
            onRedirect={(id, words) => void commands.redirect(id, words)}
          />
        </WalkPanelSection>
      ) : null}
      {said === null ? null : (
        <p role="status" className="text-fg-muted">
          {said}
        </p>
      )}
    </WalkPanel>
  );
}
