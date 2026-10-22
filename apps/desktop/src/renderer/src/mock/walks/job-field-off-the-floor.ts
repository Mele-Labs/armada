// One Job played through on its Field: its drone leaves the code floor when it is not coding. It thinks over its last file, a subagent
// lifts off it, it lands on the pad to run checks, a failing check draws a line to the file Fleet read from its log, it walks the app with a
// window over the pad, calls the forge, and waits on you. The strip over the Field says what each drone is doing. Over the
// `job-field-off-the-floor` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const field = region("Field", { exact: true });
const strip = inside(field, role("group", "Drones"));
const main = (now: RegExp) => inside(strip, button(new RegExp(`^Draw the pad on the Field: ${now.source}`)));
const helper = (now: RegExp) => inside(strip, button(new RegExp(`^Check the wire types: ${now.source}`)));

export const jobFieldOffTheFloor = walk("job-field-off-the-floor", [
  { press: deckRow("Draw the pad on the Field"), say: "The Job is open. Its Field has a pad at the near edge, right of the base" },
  { later: main(/Writing layout\.ts$/), say: "The strip over the Field: one button for each drone. The Job's drone is writing layout.ts, its laser on the file" },
  { look: main(/Thinking$/), say: "No tool call for a few seconds: the drone rises over its last file, with no beam, and three dots circle it" },
  { later: main(/Thinking$/), say: "Now a subagent starts" },
  { look: helper(/Writing events\.ts$/), say: "A smaller drone lifts off the Job's drone, joined to it by a faint line, and flies to the file it is writing" },
  { press: helper(/Writing events\.ts$/), say: "Its button frames the camera on it" },
  { later: main(/Reading wire\.ts$/), say: "The Job's drone is reading wire.ts. Now it runs checks" },
  { look: main(/Running checks$/), say: "It lands on the pad. A light for each check along the pad's front edge: two green, the tests pulsing while they run" },
  { later: main(/Running checks$/), say: "The tests fail" },
  { look: field, say: "Their light is red, and a red dashed line runs from it to layout.ts, outlined in red: Fleet read from the log that the test failed at layout.ts:142, not in the test file. Hover the file and its card says so. The drone leaves the pad" },
  { later: field, say: "A fix to layout.ts, and the checks run again" },
  { look: main(/Running checks$/), say: "Back on the pad. The lights of the checks not yet run again are unlit, and the red line is gone" },
  { later: main(/Running checks$/), say: "All three pass, and the subagent finishes" },
  { look: helper(/Done$/), say: "The subagent flies back into the Job's drone and is gone. Its button stays, greyed" },
  { later: helper(/Done$/), say: "Now the Job walks the app" },
  { look: main(/Walking the app$/), say: "Over the pad, with a window above it naming the walk. Fleet records no step or picture of it, so the frame is plain" },
  { later: main(/Walking the app$/), say: "The walk ends and the Job calls the forge" },
  { look: main(/Calling Forge$/), say: "A beam from the drone to the forge's outpost past the far edge" },
  { later: main(/Calling Forge$/), say: "The turn ends on a question for you" },
  { look: main(/Waiting on you$/), say: "Waiting on you: the drone's beacon is amber" },
]);
