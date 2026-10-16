// The flight deck's Activity stands as tall as the merge queue and the flight controls beside it. Over `activity-beeswarm`.

import { region, walk } from "../walk";

const activityHeight = walk("activity-beeswarm", [
  { look: region("Merge queue"), say: "Activity's top lines up with the merge queue's top" },
  { look: region("Activity"), say: "Its bottom lines up with Flight control's bottom, so Activity is as tall as the two together" },
]);

export { activityHeight as "activity-height" };
