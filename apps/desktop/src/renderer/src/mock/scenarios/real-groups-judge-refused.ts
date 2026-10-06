import { holding } from "../holding";
import type { Scenario } from "../moment";
import { featureJudgeRefused } from "@armada/jobs/fake";

export const s070GroupsJudgeRefused: Scenario = holding("real/groups-judge-refused", "A plan Fleet ran in groups, the last refused", [featureJudgeRefused()], {
  opens: featureJudgeRefused().job.id,
});
