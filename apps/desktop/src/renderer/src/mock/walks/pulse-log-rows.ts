// Pulse's log rows off Fleet's `logs` (#1648): what each file weighs, and
// whether a Drone is writing it now, where Open would be.

import { region, tab, text, walk } from "../walk";

export const pulseLogRows = walk("arc/executing-sequential", [
  { press: tab("Pulse"), say: "A Job whose third group is working" },
  { look: region("Job logs"), say: "Every file it has, its own log first" },
  { look: text("596.9 KiB"), say: "T5's transcript, being written, and its size" },
  { look: text("plan · a1"), say: "A Judge brief, finished, with its size and Open" },
  { look: text("01M2D5HKQP001DRONE0000T3"), say: "A file Fleet could not measure has no size" },
]);
