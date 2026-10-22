// A Job's Field drawing the whole repository: the plots it touched outlined, two other Jobs faded, the files it wrote and read glowing,
// and the slider under the plan scrubbing back through its reads and writes. Over the `job-field-whole-repo` scenario.

import { button, deckRow, inside, region, role, walk } from "../walk";

const field = region("Field", { exact: true });
const slider = inside(field, role("slider", "When in the Job"));
const large = role("group", "Expanded Field");

export const jobFieldWholeRepo = walk("job-field-whole-repo", [
  { press: deckRow("Rotate the audit log"), say: "The Job is open. Its Field in the right column opens on its two drones" },
  { look: field, say: "The floor is the whole repository and keeps its shape. The plots the Job touched have an accent outline. The file written a minute ago glows amber, the one written six minutes ago is fainter, and the files read in the last five minutes glow faintly blue" },
  { press: inside(field, button("Zoom out")), say: "Zoom out" },
  { press: inside(field, button("Zoom out")), say: "The whole repository. The drones of the two other Jobs in it are faded, with no beam and no name" },
  { hover: inside(field, role("option", /^Pin the store clock/)), say: "A faded drone still opens its card" },
  { look: slider, say: "The slider under the plan, a tick for each read and write. At the right end the Field is live" },
  { key: "ArrowLeft", on: slider, say: "Back one read or write" },
  { key: "ArrowLeft", on: slider, say: "Back another" },
  { look: inside(field, role("status", "Replay")), say: "Scrubbed back. The glow and the drone stand as they were then, and a numbered route joins its stops: dotted into a read, dashed into a write" },
  { key: "End", on: slider, say: "The right end is live again. The route is gone" },
  { key: "e", on: field, say: "E opens the Field large over the stage" },
  { look: large, say: "The same Field large: the whole repository, the glow on the files, the two faded Jobs and the slider under the plan" },
  { key: "ArrowLeft", on: inside(large, role("slider", "When in the Job")), say: "The slider scrubs here too" },
  { key: "ArrowLeft", on: inside(large, role("slider", "When in the Job")), say: "Back another" },
  { look: inside(large, role("status", "Replay")), say: "Two stops back, large: the numbered route, and the glow as it was then" },
  { key: "Escape", on: large, say: "Esc puts it back" },
]);
