// The same canvas past the gate, on a Job mid-Implement: the Overview every Job draws.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureRunning } from "../feature-running";

export const s110FeatureRunning: Scenario = holding("proto/feature-running", featureRunning().name, [featureRunning()], { opens: featureRunning().job.id });
