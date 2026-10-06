// Each task's tier and the model its Drone ran, as Fleet serves them since 23.6.

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureWithTiers } from "../job-tiers-fixture";

export const s170TiersAndModels: Scenario = holding("real/tiers-and-models", featureWithTiers().name, [featureWithTiers()], { opens: featureWithTiers().job.id });
