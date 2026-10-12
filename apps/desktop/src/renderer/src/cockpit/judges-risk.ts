// Why the Judges brought a refusal to a person rather than sending it back: a panel that agrees sends
// the work back to its Drone unless a member calls it risky, and that member's reason is what the call
// card says. Read once per card from the Job's own `get_job`, since the Board's row carries none. #2105.

import { useEffect, useState } from "react";
import type { JobDetail, JobSummary } from "@armada/protocol";

/** The risk reason on the step the Job is at, from its latest round of the Judges' answers. */
export function riskOf(detail: JobDetail): string | undefined {
  const step = detail.steps.find((one) => one.step_id === detail.job.current_step_id);
  const judged = step?.judged ?? [];
  const latest = judged.reduce((most, one) => Math.max(most, one.attempt), 0);
  const risky = judged.find((one) => one.attempt === latest && one.verdict !== "met" && (one.risky ?? "").trim() !== "");
  return risky?.risky?.trim();
}

/** The risk reason for a call about `job`, once Fleet has answered. Nothing where it gave none. */
export function useJudgesRisk(job: JobSummary | undefined, asks: boolean): string | undefined {
  const [risk, setRisk] = useState<{ jobId: string; said: string | undefined }>();
  const jobId = asks ? job?.id : undefined;
  useEffect(() => {
    if (jobId === undefined || typeof window.armada?.readJob !== "function") return;
    let current = true;
    void window.armada.readJob(jobId).then((read) => current && setRisk({ jobId, said: read.ok ? riskOf(read.value) : undefined }));
    return () => {
      current = false;
    };
  }, [jobId, job?.current_step_id]);
  return risk?.jobId === jobId ? risk?.said : undefined;
}
