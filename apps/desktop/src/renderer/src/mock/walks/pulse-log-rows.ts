// Pulse's log rows off Fleet's `logs` (#1648): each file named by where its
// Drone worked, what it weighs, a pulsing mark while a Drone writes it, and an
// Open on every transcript and every brief.

import { region, role, tab, text, walk } from "../walk";

export const pulseLogRows = walk("arc/executing-sequential", [
  { press: tab("Pulse"), say: "A Job whose third group is working" },
  { look: region("Job logs"), say: "Every file it has, its own log first" },
  { look: text("implement · T5"), say: "A transcript, named by its step and task" },
  { look: role("img", "Being written"), say: "Pulsing: T5 is writing it now" },
  { look: text("implement · T1"), say: "A finished transcript, with its size and Open" },
  { look: text("plan · a1"), say: "A Judge brief, with its size and Open" },
  { look: text("implement · T3"), say: "A file Fleet could not measure has no size" },
]);

/** A gaming check's brief, on the Job it stopped. */
export const pulseGamingBrief = walk("job/escalatedEvidenceSuspect", [
  { press: tab("Pulse"), say: "The Job a Judge refused" },
  { look: text("regression_verify · gaming check"), say: "The gaming check's brief opens too" },
]);
