// Jobs' registration: its members answer from `jobs-fleet.ts`, over this fake's handle.

import type { JobsApi, JobsState } from "../../../../shared/api/jobs";
import { JOBS_NOTHING_YET } from "../../../../shared/api/jobs";
import type { Slice } from "../fake-context";
import { jobsApi } from "../jobs-fleet";

export const jobs: Slice<JobsApi, JobsState> = {
  name: "jobs",
  state: JOBS_NOTHING_YET,
  api: (scenario, fleet) => jobsApi(scenario, fleet),
};
