// The same canvas past the gate, on a Job mid-Implement: the Overview every Job draws.

import { holding } from "../holding";
import { manifesting } from "../manifest-fake";
import type { Scenario } from "../moment";
import { featureRunning } from "@armada/jobs/fake";

export const s110FeatureRunning: Scenario = {
  ...holding("proto/feature-running", featureRunning().name, [featureRunning()], { opens: featureRunning().job.id }),
  // The Commands a step added to the Job may run are the ones this repository's Manifest declares.
  behaves: (fleet) => ({ readManifestFile: manifesting().behaves?.(fleet).readManifestFile! }),
};
