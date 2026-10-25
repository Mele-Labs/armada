// A Job's right column tells its story top to bottom: the Workflow, the Blueprint (the plan), the Field and the Landing. The panel the stage is at
// is open and the other two stand folded to their bands. Blueprint while the plan is drafted, Field while drones work, Landing at the end. The Job's
// issues are the Cockpit's Issue line and its pull requests the Merge line under it, not chips on the Landing band. Approved, the Blueprint is one
// line to each group over its tasks, the one being worked lit with what the turn is doing, and a press on a group opens its subagent type and paths.
// The composer's Artifacts button opens the list over the right column and Esc gives the story back. Over `job-story`. `job-story-spike` lands the
// same story in a findings doc and `job-story-report` in a report.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const blueprint = region("Blueprint", { exact: true });
const field = region("Field", { exact: true });
const landing = region("Landing", { exact: true });
const artifacts = region("Artifacts", { exact: true });
const issueLine = region("Issue line", { exact: true });
const mergeLine = region("Merge queue", { exact: true });

const story = walk("job-story", [
  { press: deckRow("Retry the slot lease"), say: "A Job whose plan has just been recorded" },
  { look: blueprint, say: "The plan is open: its summary, every group with its tasks, and Approve at the foot" },
  { look: inside(blueprint, button("Sketch")), say: "The sketch is a press away in the band" },
  { look: landing, say: "Field and Landing stand folded to their bands" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { look: blueprint, say: "The plan stays up, compact: each group over its tasks, with a glyph for the state of each" },
  { look: inside(blueprint, text("Backing off between attempts")), say: "The task being worked is lit, with what the turn is doing under it" },
  { press: inside(blueprint, button("Retry the lease when the pool is contended", { exact: true })), say: "A press on a group shows its subagent type and paths" },
  { look: inside(blueprint, role("combobox", /^Subagent type of Retry the lease/)), say: "The subagent type, with the paths beside it" },
  { look: field, say: "The Field opens while the drones work" },
  { look: issueLine, say: "The Job's issues stand in one row above the Merge line" },
  { later: landing, say: "The build finishes and the pull request goes to the queue" },
  { look: inside(mergeLine, button(/#2390/)), say: "The Job's pull request is framed in the Merge line among the others" },
  { look: inside(landing, button("In merge queue")), say: "Landing opens: the pull request, its Checks and its place in the merge queue" },
  { look: inside(issueLine, button(/^Issue .*2402/)), say: "An issue the Job filed joins the Issue line" },
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
  { look: landing, say: "No pull request, so nothing to merge" },
]);

const report = walk("job-story", [
  { press: deckRow("Why do slot leases expire early"), say: "An investigation whose plan has just been recorded" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { later: landing, say: "The report is written" },
  { look: inside(landing, button("Why do slot leases expire early")), say: "Landing shows the report: its title and the first lines of its conclusion" },
  { look: mergeLine, say: "No pull request, so nothing of this Job in the Merge line" },
  { press: inside(landing, button("Why do slot leases expire early")), say: "A press opens the whole report" },
  { look: dialog("slot-lease-expiry.md"), say: "It opens in the side panel" },
]);

export { story as "job-story", spike as "job-story-spike", report as "job-story-report" };
