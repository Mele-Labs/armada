// Flight control with a Job open: its views are buttons in the Job's head, before its own, and an off one keeps its seat. The owner's ask of 25 Oct 2026.
// Its tiles stay full size and scroll sideways past six, and Expand lays them six to a row for every Job until Collapse. The owner's ask of 29 Oct 2026.

import { button, inside, region, walk } from "../walk";

const list = region("Jobs", { exact: true });
const flight = region("Flight controls");
const flightControl = walk("flight-control", [
  { press: inside(list, button("Tune the flight controls")), say: "A Job with work in flight" },
  { look: flight, say: "Its tiles are full size. Past the sixth the row scrolls sideways, and the edge fades" },
  { hover: button("Expand"), say: "Expand sits beside Edit while the tiles do not all fit" },
  { press: button("Expand"), say: "Six to a row, every tile in sight" },
  { press: inside(list, button("Draft the release notes")), say: "Another Job opens expanded too: the choice is kept" },
  { press: button("Collapse"), say: "Collapse puts the tiles back on one row" },
  { press: inside(list, button("Tune the flight controls")), say: "Back to the first Job" },
  { look: button("Artifacts"), say: "Its views are buttons in the title row, before the Job's own" },
  { press: button("Artifacts"), say: "Artifacts over the thread. The head and the composer stay" },
  { press: button("Artifacts"), say: "The lit button again goes back to the thread" },
  { press: inside(list, button("Draft the release notes")), say: "A Job just begun" },
  { hover: button("Checks"), say: "Checks is off. The tooltip says why, and the button keeps its seat" },
]);

export { flightControl as "flight-control" };
