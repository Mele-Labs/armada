// The same gate on a feature Job, for the approval canvas (prototype).

import { holding } from "../holding";
import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";
import { featureAtApproval } from "@armada/jobs/fake";

export const s100FeatureAtApproval: Scenario = {
  ...holding("proto/feature-at-approval", featureAtApproval().name, [featureAtApproval()], {
    opens: featureAtApproval().job.id,
  }),
  // The Commands a step added to the Job may run are the ones this repository's Manifest declares.
  behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
};
