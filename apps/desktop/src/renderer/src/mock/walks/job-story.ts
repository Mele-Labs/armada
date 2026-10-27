// A Job's right column tells its story top to bottom in three zones: Completed, Now and Next. The plan is drafted whole in Now. The Job's
// issues are the plan's: a group that works one carries it on a meta line under its title, and the group being worked carries its issue under its live line. An issue the Job claimed
// and the ones it filed for later stand in Completed, and the follow-ups the agent kept stand in Deferred, in Next. Approved, Now lists the group being worked with its tasks and the turn's
// live line, a button opens the whole plan, and a group's title opens the issue it works. The last line of a drafted group is an empty task to type into.
// The head's Artifacts icon opens the list over the right column and Esc gives the story back. Over `job-story`. `job-story-spike` lands the
// same story in a findings doc and `job-story-report` in a report. `job-document` reads that report. `job-story-page` is the pull request
// Job's own page, from Completed and from where its thread names it.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const now = region("Now", { exact: true });
const next = region("Next", { exact: true });
const field = region("Field", { exact: true });
const landing = region("Completed", { exact: true });
const artifacts = region("Artifacts", { exact: true });
const mergeLine = region("Merge queue", { exact: true });

const story = walk("job-story", [
  { press: deckRow("Retry the slot lease"), say: "A Job whose plan has just been recorded" },
  { look: now, say: "The plan is open in Now: its summary and each group as a title over its tasks, with Approve at the foot" },
  { look: inside(now, button(/^Issue LEASE-20481/)), say: "A group that works an issue carries it on a line under its title: ticket glyph, key and the issue's title, one press" },
  { press: inside(now, button(/^Edit group Show the attempt/)), say: "The pencil opens a group" },
  { look: inside(now, role("combobox", /^Subagent type of Show the attempt/)), say: "The subagent type is one small control under the header, which holds the only title. Each task's words run the width of the group" },
  { look: inside(now, role("textbox", "New task in Show the attempt on the bay tile")), say: "The last line of every group is an empty task: type into it and Enter adds the task, with the line left empty for the next" },
  { press: inside(now, button("Approve")), say: "Approve" },
  { look: inside(now, text("Backing off between attempts")), say: "Now lists the group being worked: the task finished last in the done colour, the task being worked on with its live line, the next one waiting" },
  { look: inside(now, button(/^Issue LEASE-20481/)), say: "A long key from another tracker fits under the task, after its live line, with the issue's title cut by an ellipsis" },
  { press: inside(now, button("Whole plan")), say: "One press opens the whole plan, with a glyph for the state of every task. The same press, or ⌥N, folds it back" },
  { press: inside(now, button("Retry the lease when the pool is contended", { exact: true })), say: "A group that works an issue opens that issue from its title. The chevron beside it opens the group" },
  { look: dialog("LEASE-20481"), say: "The issue opens over the column" },
  { key: "Escape", on: dialog("LEASE-20481"), say: "Esc" },
  { look: field, say: "The Field opens while the drones work" },
  { later: landing, say: "The build finishes and the pull request goes to the queue" },
  { look: landing, say: "Completed holds what the build made" },
  { look: inside(landing, role("list", "Pull requests")), say: "Two pull requests, each framed in its state's hue with a heavier edge: the number in that hue and the title, then the branch as a mono chip, the Checks and the queue state as filled chips. The act stands in a fixed column at the right, centred on its row" },
  { look: inside(landing, role("img", "In the merge queue, merges when its turn comes")), say: "The first is in the merge queue and has no act, its column kept. The second is not queued: its Checks run, and its act is an icon button" },
  { look: inside(landing, button(/^Issue .*2395/)), say: "An issue the Job claimed stands among Completed's pull requests as a row of its own, and a press opens it" },
  { press: inside(next, button("Deferred")), say: "Deferred, in Next, opens the follow-up the agent kept" },
  { look: inside(landing, button(/^Issue .*2402/)), say: "An issue the Job filed for later stands with the other issues in Completed, and a press opens it" },
  { press: button("Artifacts", { exact: true }), say: "The Artifacts icon in the Job's head" },
  { look: inside(artifacts, role("group", "View")), say: "List and Graph are two icon toggles beside the search, each with a tooltip naming it and V. Back is at the far right of the head" },
  { look: inside(artifacts, button(/^Pull request #2391/)), say: "Every row has two lines: the title, which wraps before it is cut, and under it the branch or path, the kind and its states. A pull request is the same row as in Completed" },
  { key: "Escape", on: artifacts, say: "Esc" },
  { look: landing, say: "The story is back as it was" },
]);

const spike = walk("job-story", [
  { press: deckRow("Measure lease contention"), say: "A spike whose plan has just been recorded" },
  { look: now, say: "Now holds the plan, as for any Job" },
  { press: inside(now, button("Approve")), say: "Approve" },
  { later: landing, say: "The findings are written up" },
  { look: inside(landing, button("Lease contention findings")), say: "A spike lands in its findings doc, which opens as an artifact does" },
  { look: landing, say: "No pull request, so nothing to merge" },
]);

const report = walk("job-story", [
  { press: deckRow("Why do slot leases expire early"), say: "An investigation whose plan has just been recorded" },
  { press: inside(now, button("Approve")), say: "Approve" },
  { later: landing, say: "The report is written" },
  { look: inside(landing, button("Why do slot leases expire early")), say: "Completed shows the report: its title and the first lines of its conclusion" },
  { look: mergeLine, say: "No pull request, so nothing of this Job in the Merge line" },
  { press: inside(landing, button("Why do slot leases expire early")), say: "A press opens the whole report" },
  { look: region("Document", { exact: true }), say: "It opens in a panel over the Job" },
]);

// A Job that lands a pull request and also wrote a page with the document skill: the page stands in its Completed beside the pull requests.
const page = walk("job-story", [
  { press: deckRow("Retry the slot lease"), say: "A Job whose workflow lands a pull request" },
  { press: inside(now, button("Approve")), say: "Approve" },
  { later: landing, say: "The build finishes, and the Job has written a page beside its pull request" },
  { look: inside(landing, button(/^Why do slot leases expire early/)), say: "The page stands in Completed with the pull requests, as a spike's findings do" },
  { press: inside(landing, button(/^Why do slot leases expire early/)), say: "A press opens it" },
  { look: region("Document", { exact: true }), say: "In a panel over the Job: Contents with the Notes under it, the page beside them" },
  { key: "Escape", on: region("Document", { exact: true }), say: "Esc" },
  { look: landing, say: "The Job is back as it was" },
  { look: inside(region("Thread"), role("link", /^Why do slot leases expire early/)), say: "Where the Job names the page in its thread, the path reads as the page's title with a document glyph" },
  { press: inside(region("Thread"), role("link", /^Why do slot leases expire early/)), say: "A press opens the same panel, not the raw file" },
  { look: region("Document", { exact: true }), say: "The page, over the Job" },
  { key: "Escape", on: region("Document", { exact: true }), say: "Esc" },
]);

export { story as "job-story", spike as "job-story-spike", report as "job-story-report", page as "job-story-page" };
