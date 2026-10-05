// The same canvas with a gate stage in every kind and state, staged for the owner's walk (5 Oct 2026).

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureGates } from "../feature-gates";

export const s120GatesOnTheSpine: Scenario = holding("proto/gates-on-the-spine", featureGates().name, [featureGates()], { opens: featureGates().job.id });
