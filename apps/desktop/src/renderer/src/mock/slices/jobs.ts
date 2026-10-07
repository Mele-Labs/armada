// Jobs' registration: its members come from `@armada/jobs/fake`, over this fake's handle.

import { jobsApi } from "@armada/jobs/fake";

import type { JobsApi, JobsState } from "../../../../shared/api/jobs";
import { JOBS_NOTHING_YET } from "../../../../shared/api/jobs";
import type { Slice } from "../fake-context";
import { proposingRow } from "../fake-context";

export const jobs: Slice<JobsApi, JobsState> = {
  name: "jobs",
  state: JOBS_NOTHING_YET,
  api: (scenario, fleet) => jobsApi(scenario, { ...fleet, proposingRow }),
};
