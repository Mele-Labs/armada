// The same gate on a feature Job, for the approval canvas (prototype).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureAtApproval } from "../feature-at-approval";

export const s100FeatureAtApproval: Scenario = holding("proto/feature-at-approval", featureAtApproval().name, [featureAtApproval()], {
  opens: featureAtApproval().job.id,
});
