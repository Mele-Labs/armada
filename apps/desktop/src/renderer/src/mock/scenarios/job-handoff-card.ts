// The ask on a Job's call card with its four answers, and the second question that "Remind me at a higher limit" brings forward. The walk `jobHandoffCard` plays it. Built in `../job-handoff.ts`.

import { jobHandoff } from "../job-handoff";
import type { Scenario } from "../moment";

export const s297JobHandoffCard: Scenario = jobHandoff("job-handoff-card", "A Job asks to carry on in a new one, with four answers");
