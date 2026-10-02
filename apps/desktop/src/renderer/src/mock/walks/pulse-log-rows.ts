// Pulse's log rows off Fleet's `logs` (#1648): each file named by where its
// Drone worked, what it weighs, a pulsing mark while a Drone writes it, an
// Open on every transcript and brief, and a panel that reads a file live.

import { button, dialog, inside, region, role, tab, text, walk } from "../walk";

export const pulseLogRows = walk("arc/executing-sequential", [
  { press: tab("Pulse"), say: "A Job whose third group is working" },
  { press: button("Drone transcript, implement · T5"), say: "T5's transcript opens in a panel, still being written" },
  { look: dialog("Drone transcript"), say: "It follows the tail" },
  { look: text("Running the screens tests against the new row."), say: "A new row arrives as T5 writes it" },
  { press: inside(dialog("Drone transcript"), button("Close")), say: "Back to the list" },
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
