// Workflow: the Session's workflow in a one-line panel between the Field and Progress. Its picker (or its name) and its steps stand in the band, and the
// panel has no body. It is drawn only where there is a workflow or a picker to show: a Session with the rail off has none.

import type { SessionWork, SessionWorkActs } from "@armada/screens/src/draft/sessions";

import { Flow, StepRail } from "./StatusPanel";
import { SvPanel, type Place } from "./SvPanel";

export function WorkflowPanel({ work, place, id, manifestId, acts }: { work: SessionWork; place: Place; id: string; manifestId?: string | undefined; acts: SessionWorkActs | undefined }) {
  const flow = work.workflow;
  if (work.railOff === true || (flow === undefined && (work.proposal === undefined || acts === undefined))) return null;
  const rail = (
    <>
      <Flow work={work} acts={acts} id={id} manifestId={manifestId} />
      {flow === undefined ? null : <StepRail steps={flow.steps} at={flow.at} />}
    </>
  );
  return <SvPanel label="Workflow" area="workflow" place={place} rail={rail} />;
}
