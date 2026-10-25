// A Job's right column tells its story top to bottom: the Workflow, the Blueprint (the plan), the Field and the Landing. The panel the stage is at
// is open and the other two stand folded to their bands. Blueprint while the plan is drafted, Field while drones work, Landing at the end, where
// the issue and pull request chips stand on the band whether it is open or folded. The composer's Artifacts button opens the list over the right
// column and Esc gives the story back. Over `job-story`. `job-story-spike` lands the same story in a findings doc.

import { button, deckRow, inside, region, role, walk } from "../walk";

const blueprint = region("Blueprint", { exact: true });
const field = region("Field", { exact: true });
const landing = region("Landing", { exact: true });
const artifacts = region("Artifacts", { exact: true });

const story = walk("job-story", [
  { press: deckRow("Retry the slot lease"), say: "A Job whose plan has just been recorded" },
  { look: blueprint, say: "The plan is open: its summary, every group with its tasks, and Approve at the foot" },
  { look: inside(blueprint, button("Sketch")), say: "The sketch is a press away in the band" },
  { look: landing, say: "Field and Landing stand folded to their bands" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { look: blueprint, say: "The plan stays up, compact: a line to each group and a mark for each task" },
  { look: field, say: "The Field opens while the drones work, the turn's account above the map" },
  { look: inside(landing, button("Pull request #2390", { exact: true })), say: "The draft pull request is already a chip on the Landing band" },
  { later: landing, say: "The build finishes and the pull request goes to the queue" },
  { look: inside(landing, button("In merge queue")), say: "Landing opens: the pull request, its Checks and its place in the merge queue" },
  { look: inside(landing, button(/^Issue .*2402/)), say: "An issue the Job filed is a chip on the band beside the pull request" },
  { look: inside(landing, region("Watching")), say: "Watching stands inside Landing" },
  { look: inside(landing, region("Later")), say: "So does Later" },
  { press: button("Show artifacts"), say: "The composer's Artifacts button" },
  { look: artifacts, say: "The list covers the right column and the story steps aside" },
  { key: "Escape", on: artifacts, say: "Esc" },
  { look: landing, say: "The story is back as it was" },
]);

const spike = walk("job-story", [
  { press: deckRow("Measure lease contention"), say: "A spike whose plan has just been recorded" },
  { look: blueprint, say: "Blueprint is open, as for any Job" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { later: landing, say: "The findings are written up" },
  { look: inside(landing, role("list", "Findings")), say: "A spike lands in its findings doc, which opens as an artifact does" },
  { look: landing, say: "No pull request, so no chip on the band and no merge" },
]);

export { story as "job-story", spike as "job-story-spike" };
