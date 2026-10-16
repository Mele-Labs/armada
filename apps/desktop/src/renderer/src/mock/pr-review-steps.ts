// What the two review walks share: everything from the list on. They differ only in how the review surface is reached
// (the Cockpit's Flight control, or a tab of its own) and in what they do on leaving it. Not under `walks/`, where
// every file is a walk.

import { button, inside, region, role, tab } from "./walk";
import type { Step } from "./walk";

const list = region("Pull requests");
const agent = region("Agent review");
const yours = region("Your review");
const reader = (number: number) => region(`Pull request #${number}`);
const thread = (file: string) => role("article", `Thread on ${file}`);
const finding = (title: string) => role("article", `Finding: ${title}`);
const viewed = (path: string) => role("checkbox", `Viewed ${path}`);

export const toTheList: Step[] = [
  { look: list, say: "The pull requests waiting on him, fetched as the view opened" },
  { hover: inside(list, button("Refresh")), say: "Refresh names when the list was last fetched. Nothing is watched in between" },
  { look: inside(list, tab("Review requested")), say: "Review requested: pull requests someone asked him to review" },
  { press: inside(list, tab("Mine")), say: "Mine: pull requests his own agents opened" },
  { look: inside(list, button(/#1896/)), say: "Each row: its title, review state, checks, number, author and repository" },
  { hover: inside(list, role("img", "Checks failed")), say: "A state is a glyph, named on hover" },
  { press: inside(list, tab("Review requested")), say: "Back to review requested" },
];

export const theReview: Step[] = [
  { press: inside(list, button(/#1899/)), say: "Open one a teammate asked for" },
  { look: inside(reader(1899), role("navigation", "File tree")), say: "The whole pull request in one scroll, with a tree to jump by" },
  { press: inside(inside(reader(1899), role("navigation", "File tree")), button("crates/fleet/src/plan_usage.rs")), say: "A file in the tree scrolls the diff to it" },
  { look: inside(reader(1899), thread("plan_usage.rs")), say: "Threads already on the pull request sit on their lines" },
  { press: inside(reader(1899), viewed("docs/concepts/plan-usage.md")), say: "Viewed folds a file once it is read" },
  { press: inside(thread("plan_usage.rs"), button("Reply")), say: "Replying opens the box under the thread, in the diff" },
  { type: "A const is fine. Name the unit in the doc comment.", into: inside(thread("plan_usage.rs"), role("textbox", "Reply")), say: "A reply" },
  { press: inside(thread("plan_usage.rs"), button("Add to review")), say: "It joins the pending review. Nothing is posted yet" },
  { look: inside(thread("plan_usage.rs"), role("article", "Pending comment")), say: "A pending comment is dashed and his" },
  { press: inside(reader(1899), button(/Comment on .*self\.client\.windows\(\)/)), say: "A press in a line's gutter starts a comment on it" },
  { type: "Take the lock after the call, so a slow read does not hold it.", into: inside(reader(1899), role("textbox", "Comment")), say: "A line comment" },
  { press: inside(inside(reader(1899), role("form", "Comment")), button("Add to review")), say: "It joins the pending review too" },
  { look: inside(yours, role("list", "Pending comments")), say: "Both wait in his review, to be posted together" },
  { press: inside(agent, button("Review with an agent")), say: "An agent reviews only when he presses for one" },
  { look: inside(agent, button("Reviewing")), say: "Running, in the button" },
  { look: inside(reader(1899), finding("A reset inside the minute is served late")), say: "Findings are pinned to lines. Hatched, they are Armada's and are not on the forge" },
  { hover: inside(finding("A reset inside the minute is served late"), role("img", "Armada finding, not posted")), say: "Named on hover" },
  { look: inside(agent, role("list", "Findings")), say: "The same findings in the column, each one a jump" },
  { press: inside(finding("Is read_at required?"), button("Dismiss finding")), say: "Dismiss puts one away" },
];

export const theSessionsOwn: Step[] = [
  { press: inside(list, tab("Mine")), say: "Mine" },
  { press: inside(list, button(/#1896/)), say: "One his Session holds" },
  { look: inside(reader(1896), button("Session: Fix the reconnect wait")), say: "The Session that holds it is in the head" },
  { press: inside(agent, button("Review with an agent")), say: "An agent review of this one" },
  { look: inside(reader(1896), finding("The attempt count is never reset")), say: "A finding on its line" },
  { look: inside(finding("The attempt count is never reset"), button(/Send to Session/)), say: "Send to Session is there because a Session holds it" },
  { press: inside(finding("The attempt count is never reset"), button(/Send to Session/)), say: "The finding goes to the Session as a message" },
  { look: inside(finding("The attempt count is never reset"), role("img", "Sent to the Session")), say: "Sent once" },
  { press: inside(yours, tab("Request changes")), say: "Request changes" },
  { type: "The count has to reset when the socket opens. Cap the jitter too.", into: inside(yours, role("textbox", "Summary")), say: "The summary" },
  { press: inside(yours, button("Submit review")), say: "Submit posts the pending comments and the verdict together" },
  { look: inside(yours, role("list", "Submitted reviews")), say: "Recorded here" },
  { look: inside(list, role("img", "Changes requested")), say: "And on the row" },
];

