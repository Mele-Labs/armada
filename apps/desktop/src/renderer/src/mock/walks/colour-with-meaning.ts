// Colour that says something on the pages outside the Cockpit. A design mock: nothing here asserts a colour.

import { button, viaPalette, walk } from "../walk";

const colourWithMeaning = walk("colour-with-meaning", [
  ...viaPalette("Usage", "Usage. Figures going up light their tile; going down get a green chip"),
  ...viaPalette("Doctor", "Doctor. A warning or a failure heads its well in its hue, as a bay does"),
  ...viaPalette("Checks", "Checks. Glyphs at full strength; running and failed name themselves in their hue"),
  ...viaPalette("Issues", "Issues. A Job's issue carries a bar and a ground; the rest stay neutral"),
  ...viaPalette("Reviews", "Reviews. A pull request's checks run down the row's edge"),
  { press: button("#212", {}), say: "A picked row keeps its edge" },
  ...viaPalette("Projects", "Projects. Needs you and In a Job take their hue; a task that asks gets the bar"),
  ...viaPalette("Limits", "Limits. Modified takes the slider's hue"),
]);

export { colourWithMeaning as "colour-with-meaning" };
