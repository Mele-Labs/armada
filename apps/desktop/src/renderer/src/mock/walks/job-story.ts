// A Job's right column tells its story top to bottom: the Workflow, the Blueprint (the plan), the Field and the Landing. The panel the stage is at
// is open and the other two stand folded to their bands. Blueprint while the plan is drafted, Field while drones work, Landing at the end. The Job's
// issues are the Cockpit's Issue line and its pull requests the Merge line under it, not chips on the Landing band; an issue a pull request closes
// carries it, and hovering either chip lights the other. Approved, the Blueprint reads as Now (the task finished last, the ones being worked with the turn's
// live line, the next one), a button opens the whole plan, and a press on a group opens its subagent type and paths.
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
  { look: blueprint, say: "The plan is open: its summary and each group as a title over its tasks, with Approve at the foot" },
  { press: inside(blueprint, button(/^Edit group Show the attempt/)), say: "The pencil opens a group" },
  { look: inside(blueprint, role("combobox", /^Subagent type of Show the attempt/)), say: "Its subagent type and paths show now, and its tasks stay editable" },
  { look: landing, say: "Field and Landing stand folded to bands as short as the Workflow's" },
  { press: inside(blueprint, button("Approve")), say: "Approve" },
  { look: inside(blueprint, text("Backing off between attempts")), say: "The Blueprint reads as Now: the task finished last, the task being worked with its live line, the next one waiting" },
  { press: inside(blueprint, button("Whole blueprint")), say: "One press opens the whole blueprint, with a glyph for the state of every task. The same press, or ⌥N, folds it back to Now" },
  { press: inside(blueprint, button("Retry the lease when the pool is contended", { exact: true })), say: "A press on a group shows its subagent type and paths" },
  { look: inside(blueprint, role("combobox", /^Subagent type of Retry the lease/)), say: "The subagent type, with the paths beside it" },
  { look: field, say: "The Field opens while the drones work" },
  { look: inside(issueLine, button(/^Issue .*2361/)), say: "An issue a pull request closes ends in that pull request's glyph and number" },
  { later: landing, say: "The build finishes and the pull request goes to the queue" },
  { hover: inside(issueLine, button(/^Issue .*2361/)), say: "Hovering the issue lights its pull request in the Merge line, and hovering that lights the issues it closes" },
  { look: inside(issueLine, button(/^Issue .*2374/)), say: "An issue with no pull request carries no mark" },
  { look: inside(landing, button("In merge queue")), say: "Landing opens: the pull request, its Checks and its place in the merge queue" },
  { look: inside(landing, region("Deferred")), say: "Deferred stands inside Landing" },
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
