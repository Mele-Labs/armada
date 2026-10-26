// A Job's right column tells its story top to bottom: the Workflow, the Blueprint (the plan), the Field and the Landing. The panel the stage is at
// is open and the other two stand folded to their bands. Blueprint while the plan is drafted, Field while drones work, Landing at the end. The Job's
// issues are the Blueprint's: a group that works one carries its chip, and the task being worked in the Now view carries its group's. The issues the Job filed
// for later stand in Deferred. Approved, the Blueprint reads as Now (the task finished last, the ones being worked with the turn's
// live line, the next one), a button opens the whole plan, and a press on a group opens its subagent type and paths as one row of meta. One open panel fills the column.
// The composer's Artifacts button opens the list over the right column and Esc gives the story back. Over `job-story`. `job-story-spike` lands the
// same story in a findings doc and `job-story-report` in a report.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const blueprint = region("Blueprint", { exact: true });
const field = region("Field", { exact: true });
const landing = region("Landing", { exact: true });
const artifacts = region("Artifacts", { exact: true });
const mergeLine = region("Merge queue", { exact: true });

const story = walk("job-story", [
  { press: deckRow("Retry the slot lease"), say: "A Job whose plan has just been recorded" },
  { look: blueprint, say: "The plan is open: its summary and each group as a title over its tasks, with Approve at the foot" },
  { look: inside(blueprint, button(/^Issue .*2361/)), say: "A group that works an issue carries its chip in the header, with the title in the tooltip" },
  { press: inside(blueprint, button(/^Edit group Show the attempt/)), say: "The pencil opens a group" },
  { look: inside(blueprint, role("combobox", /^Subagent type of Show the attempt/)), say: "The subagent type and the paths are one row of small meta under the header, which holds the only title. The tasks are set as the Now view's" },
  { look: landing, say: "Field and Landing stand folded to bands as short as the Workflow's" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { look: inside(blueprint, text("Backing off between attempts")), say: "The Blueprint reads as Now: the task finished last, the task being worked with its live line and its group's issue chip, the next one waiting" },
  { press: inside(blueprint, button("Whole blueprint")), say: "One press opens the whole blueprint, with a glyph for the state of every task. The same press, or ⌥N, folds it back to Now" },
  { press: inside(blueprint, button("Retry the lease when the pool is contended", { exact: true })), say: "A press on a group shows its subagent type and paths as one row" },
  { look: field, say: "The Field opens while the drones work" },
  { later: landing, say: "The build finishes and the pull request goes to the queue" },
  { look: landing, say: "Landing is the one open panel and fills the column to the bottom" },
  { look: inside(landing, button("In merge queue")), say: "The pull request, its Checks and its place in the merge queue" },
  { press: inside(landing, button("Deferred")), say: "Deferred opens" },
  { look: inside(landing, button(/^Issue .*2402/)), say: "An issue the Job filed for later stands beside the deferred items, and a press opens it" },
  { press: button("Show artifacts"), say: "The composer's Artifacts button" },
  { look: inside(artifacts, role("searchbox", "Search artifacts")), say: "The head holds the title and Back. The search fills a row of its own, with the List and Graph switch beside it" },
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
