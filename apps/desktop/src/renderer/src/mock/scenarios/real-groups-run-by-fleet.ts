import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureRunInGroups } from "../job-groups-fixture";

export const s060GroupsRunByFleet: Scenario = holding("real/groups-run-by-fleet", "A plan Fleet ran in groups, the last red", [featureRunInGroups()], {
  opens: featureRunInGroups().job.id,
});
