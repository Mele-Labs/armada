// The same ask, and the new Job does not come up: the old Job stays as it was and keeps its row. The walk `jobHandoffFails` plays it. Built in `../job-handoff.ts`.

import { jobHandoff } from "../job-handoff";
import type { Scenario } from "../moment";

export const s296JobHandoffFails: Scenario = jobHandoff("job-handoff-fails", "A Job asks to carry on in a new one, and the new Job does not start", true);
