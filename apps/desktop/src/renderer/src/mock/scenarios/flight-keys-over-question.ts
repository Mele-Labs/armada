// A Job open with its question in Needs you, and the flight controls below it. The walk `flightKeysOverQuestion` plays it. Built in `../job-handoff.ts`.

import { jobHandoff } from "../job-handoff";
import type { Scenario } from "../moment";

export const s322FlightKeysOverQuestion: Scenario = jobHandoff("flight-keys-over-question", "A Job open with a question, and the flight control keys beside its numbers");
