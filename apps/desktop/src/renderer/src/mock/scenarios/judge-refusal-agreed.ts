// A Judge refusal he agreed with: the step stopped and the Job escalated,
// with Fleet's recourse in the lead and the step panel (Job 3, 2 Oct 2026).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureAfterAgreeing } from "@armada/jobs/fake";

export const s140RefusalAgreed: Scenario = holding("judge/refusal-agreed", "A Judge refusal agreed with, the step stopped", [featureAfterAgreeing()], {
  opens: featureAfterAgreeing().job.id,
});
