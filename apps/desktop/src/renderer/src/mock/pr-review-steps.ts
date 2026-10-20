// What the two review walks share: everything from the list on. They differ only in how the review surface is reached
// (the Cockpit's Flight control, or a tab of its own) and in what they do on leaving it. Not under `walks/`, where
// every file is a walk.

import { button, dialog, inside, place, region, role, tab, text } from "./walk";
import type { Step } from "./walk";

const list = region("Pull requests");
const flight = region("Flight controls");
const yours = region("Your review");
const reader = (number: number) => region(`Pull request #${number}`);
const theme = (title: string) => region(`Theme: ${title}`);
const change = (title: string) => role("article", `Change: ${title}`);
const thread = (title: string, by: string) => role("article", `Thread on ${title} by ${by}`);
const finding = (title: string) => role("article", `Finding: ${title}`);
const tool = (name: string) => inside(flight, button(name));
const results = (name: string) => region(`${name} results`);
const asked = (words: RegExp) => inside(inside(region("Quick actions"), role("list", "Asked")), role("listitem", words));
const explanation = (name: string) => inside(region("Quick actions"), tab(name));

const pick = (name: string | RegExp) => role("menuitem", name);

/** A row opens the pull request in the side panel, on its Review tab, over the reader the Reviews page keeps; this puts the panel away. */
export const putAway = (number: number, say = "The side panel is away, and the reader is under it"): Step => ({ press: inside(dialog(`Pull request #${number}`), button(/^Close/)), say });
const lists = inside(list, button("List"));

export const toTheList: Step[] = [
  { look: list, say: "The pull requests waiting on him, fetched as the view opened" },
  { hover: inside(list, button("Refresh")), say: "Refresh names when the list was last fetched. Nothing is watched in between" },
  { press: lists, say: "One dropdown chooses the list. He can be asked to review in two ways, and both are here" },
  { press: pick("Requested for Review"), say: "Requested for Review: someone asked him by name" },
  { press: lists, say: "The dropdown again" },
  { press: pick("Team"), say: "Team: a team he is on was asked" },
  { look: inside(list, role("note", "@Mele-Labs/bridge")), say: "Each row names the team that was asked" },
  { look: inside(list, button(/#1899/)), say: "A pull request can be on two lists. This one asked him by name and asked his team" },
  { press: lists, say: "The dropdown again" },
  { press: pick("Assigned"), say: "Assigned: pull requests the forge assigned to him" },
  { look: inside(list, button(/#1899/)), say: "The same pull request, here too" },
  { press: lists, say: "The dropdown again" },
  { press: pick("My PRs"), say: "My PRs: pull requests his own agents opened" },
  { press: lists, say: "Natural Search is the last entry. It is not a box on the list" },
  { press: pick("Natural Search"), say: "Choosing it puts the box in place of the list" },
  { type: "PRs that touch files I've touched", into: inside(list, role("textbox", "Natural Search")), say: "What he wants, in his own words" },
  { press: inside(list, button("Search")), say: "An agent looks only when he sends, because it costs" },
  { look: dialog("Natural Search"), say: "It spends tokens, so it asks first. Every act that spends asks the same way, with the same box" },
  { press: inside(dialog("Natural Search"), button("Run")), say: "Run" },
  { look: inside(list, role("img", "Looking")), say: "Running, in the list" },
  { look: inside(list, role("note", /which you changed on 14 Oct/)), say: "What it found, each with one line on why it matched" },
  { press: lists, say: "The find is saved in the dropdown under his words" },
  { look: pick(/files I've touched/), say: "Run again and Remove are on the list's head" },
  { press: pick("Requested for Review"), say: "Back to Requested for Review" },
];

/** A teammate's pull request: its description comes first, and each explanation is a choice that runs once. */
export const theExplanation: Step[] = [
  { press: lists, say: "A longer one first, from the team's list" },
  { press: pick("Team"), say: "Team" },
  { key: "l", on: list, say: "L goes to the list, as it does on the Cockpit. Everything from here is the keyboard" },
  { key: "ArrowDown", on: inside(list, button(/#1899/)), say: "↓ moves down the list. J does too" },
  { key: "ArrowDown", on: inside(list, button(/#1903/)), say: "Down again" },
  { key: "Enter", on: inside(list, button(/#1907/)), say: "↵ opens a pull request with a long description" },
  putAway(1907, "It opens in the side panel on its Review tab. Closing it leaves the reader the Reviews page keeps"),
  { look: inside(reader(1907), region("Quick actions")), say: "The quick actions are above the description, because descriptions are usually long" },
  { look: inside(reader(1907), region("Description")), say: "With nothing chosen the description is open" },
  { key: "Shift+Q", on: list, say: "⇧Q goes to the quick actions bar, from any part. Its key is on its label" },
  { key: "j", on: explanation("How it works"), say: "J moves down the bar, as it does a list. So do ↓, → and K back up" },
  { key: "j", on: explanation("User Impact"), say: "Down again" },
  { key: "Enter", on: explanation("How to Test"), say: "↵ or O chooses. No kept result, so it asks first" },
  { look: dialog("How to Test"), say: "Running it uses tokens. Focus is in the box" },
  { key: " ", on: dialog("How to Test"), say: "Space turns the question off for every act that spends tokens. It is a setting on this machine" },
  { key: "Enter", on: dialog("How to Test"), say: "↵ runs. Esc would cancel" },
  { look: inside(reader(1907), region("How to Test")), say: "The explanation fills the body" },
  { hover: inside(inside(reader(1907), role("listitem", "Step 1")), role("img", "From the description")), say: "How to Test follows the author's own directions first. Each step says where it came from" },
  { hover: inside(inside(reader(1907), role("listitem", "Step 3")), button(/crates\/fleet\/README/)), say: "The description does not cover this area, so the step comes from the nearest README. Its path opens it" },
  { look: inside(reader(1907), button("Show the description")), say: "The description has tucked away into one row" },
  { key: "Shift+D", on: reader(1907), say: "⇧D opens it again when he wants to read it" },
  { look: inside(reader(1907), region("Description")), say: "Open, above the explanation" },
  { key: "Enter", on: explanation("User Impact"), say: "↵ on User Impact. The question is off now, so it runs without asking" },
  { look: inside(reader(1907), role("listitem", "Restarting Fleet")), say: "What a user sees differently, before and after" },
  { key: "Escape", on: reader(1907), say: "Esc backs out a level: from the reader to the list" },
  { key: "Shift+L", on: list, say: "⇧L opens the lists" },
  { look: pick("Requested for Review"), say: "The menu answers its own arrows and ↵. Natural Search is /" },
  { press: pick("Requested for Review"), say: "Requested for Review" },
  { press: inside(list, button(/#1899/)), say: "Open one a teammate asked for" },
  putAway(1899),
  { look: inside(reader(1899), region("Description")), say: "The author's own description" },
  { look: explanation("How to Test"), say: "A choice of explanations above it. Nothing has run on this one" },
  { press: explanation("How to Test"), say: "How to Test. An agent reads the pull request, once" },
  { look: inside(reader(1899), region("How to Test")), say: "Numbered steps, each with what to look for" },
  { look: inside(reader(1899), role("note", /One provider call a minute/)), say: "What to look for sits under the step" },
  { hover: inside(reader(1899), role("img", "Explained by an agent when you chose it")), say: "Where the explanation came from" },
  { look: inside(inside(reader(1899), region("How to Test")), role("img", /^Kept/)), say: "It is kept for the commit it read, as a tool's result is" },
  { press: explanation("User Impact"), say: "What it changes for you, as someone who uses it" },
  { look: inside(reader(1899), role("listitem", "The plan meters in the title bar")), say: "What they see or can do differently, before and after" },
  { press: explanation("How to Test"), say: "Back to how to test it" },
  { look: inside(reader(1899), role("list", "Steps")), say: "Shown at once. Nothing runs and nothing is spent" },
  { press: explanation("Why"), say: "Why it was needed" },
  { look: inside(reader(1899), button(/#1871/)), say: "The problem, what it cost, and the issue the pull request names" },
  { hover: inside(inside(reader(1899), region("Why")), role("img", "Armada's tool")), say: "Explanations are defined where tools are: Armada's four, the Kit, and the repository" },
  { look: inside(reader(1899), role("textbox", "Ask about this PR")), say: "Ask is not a quick action. It is a composer at the foot of the reader, there while a pull request is open" },
  { look: inside(reader(1899), role("list", "Asked")), say: "A question asked before is kept in a small panel under the quick actions, under his words" },
  { key: "Shift+A", on: reader(1899), say: "⇧A focuses the composer from any part. Its key is beside it" },
  { type: "Does this change what happens when gh is signed out?", into: inside(reader(1899), role("textbox", "Ask about this PR")), say: "Whatever he wants to know about this one" },
  { key: "Enter", on: inside(reader(1899), role("textbox", "Ask about this PR")), say: "↵ sends and ⇧↵ breaks the line. An agent answers only when he sends, because it costs" },
  { look: inside(reader(1899), role("img", "Answering")), say: "Running, in place" },
  { look: inside(asked(/Does this change what happens/), role("img", /^Kept/)), say: "The question joins the panel, and its answer is kept for the commit it read" },
  { hover: inside(asked(/Does this change what happens/), button("Remove this question")), say: "Run again asks it once more. Remove drops it" },
  { key: "o", on: inside(asked(/What happens to the cached read/), button(/What happens to the cached read/)), say: "O shows an earlier question's answer, as ↵ does, and at once" },
  { look: region("Answer"), say: "The answer, in the body, with nothing run" },
  { look: tool("Risks"), say: "The review tools are tiles in the Flight control, and they work on any pull request" },
  { hover: inside(tool("Risks"), role("img", "Armada's tool")), say: "A tool shows where it is defined. This one is Armada's" },
  { press: tool("Risks"), say: "Risks runs for this pull request only when pressed" },
  { look: inside(results("Risks"), finding("A second watch loses the first interval")), say: "Not explained yet, so a result is pinned to a file and its lines" },
  { look: inside(tool("Risks"), role("img", /^Kept/)), say: "A finished result is kept: the tile says when it ran and on which commit" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
  { press: tool("Risks"), say: "Pressed again, it shows at once. Nothing runs and nothing is spent" },
  { look: inside(results("Risks"), finding("A second watch loses the first interval")), say: "The same result, kept" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
  { press: explanation("How it works"), say: "How it works is the themes view. An agent reads it the first time, like the rest" },
  { look: inside(reader(1899), region("How it works")), say: "What it does in a few plain sentences, under the choice" },
  { look: theme("Fleet keeps the last read"), say: "Its changes by what they make happen, with the order they were made in" },
  { look: inside(theme("Fleet keeps the last read"), change("Hold the last read for a minute")), say: "Each change says why it was made" },
  { press: inside(change("Hold the last read for a minute"), button("Show change")), say: "The hunk is folded until he opens it" },
  { look: inside(change("Hold the last read for a minute"), thread("Hold the last read for a minute", "devon")), say: "A thread already on the line sits there" },
];

export const theTools: Step[] = [
  { press: tool("Risks"), say: "Risks again, now that it is explained" },
  { look: inside(results("Risks"), theme("Bridge asks once a minute")), say: "The same result sits under its theme and then its change" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
  { press: tool("Possible bugs"), say: "Possible bugs" },
  { look: inside(results("Possible bugs"), finding("A reset inside the minute is served late")), say: "One bug, on the change that serves the read" },
  { press: tool("Comments"), say: "Comments is the forge's conversation and line comments. It runs no agent" },
  { look: inside(reader(1899), yours), say: "His pending review is here, with Submit" },
  { press: inside(reader(1899), button("Comment on Serve it while it is fresh")), say: "A comment on a change" },
  { type: "Take the lock after the call, so a slow read does not hold it.", into: inside(reader(1899), role("textbox", "Comment")), say: "Written where the change is" },
  { press: inside(inside(reader(1899), role("form", "Comment")), button("Add to review")), say: "It joins the pending review. Nothing is posted yet" },
  { look: inside(yours, role("list", "Pending comments")), say: "Pending comments wait here to be posted together" },
  { type: "The lock should be taken after the call. Fix that and the reset case.", into: inside(yours, role("textbox", "Summary")), say: "The summary" },
  { press: inside(yours, tab("Request changes")), say: "Request changes" },
  { press: inside(yours, button("Submit review")), say: "Submit posts the pending comments and the verdict together" },
  { look: inside(yours, role("list", "Submitted reviews")), say: "Recorded here" },
  { look: inside(list, role("img", "Changes requested")), say: "And on the row" },
  { press: inside(list, button(/Bump the manifest icons/)), say: "A pull request a tool has nothing to say about" },
  { press: tool("Risks"), say: "Risks, on this one" },
  { look: inside(results("Risks"), role("img", "Nothing found")), say: "Nothing found is a small picture in the tool's colour, with what was checked under it" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
];

/** One of his Sessions holds the pull request, so it knows why and the explanation is there at open. */
export const theSessionsOwn: Step[] = [
  { press: lists, say: "The dropdown again" },
  { press: pick("My PRs"), say: "My PRs" },
  { press: inside(list, button(/#1896/)), say: "One his Job holds" },
  putAway(1896),
  { look: inside(reader(1896), button("Job: Fix the reconnect wait")), say: "The Job that holds it is in the head" },
  { look: inside(reader(1896), region("Description")), say: "The description is tucked away, since an explanation is already shown" },
  { look: inside(reader(1896), region("How it works")), say: "How it works is already under it" },
  { hover: inside(inside(reader(1896), region("How it works")), role("img", /^Kept/)), say: "Kept for the commit it read, so nothing is run to get it" },
  { press: explanation("Why"), say: "Why was kept the same way" },
  { look: inside(reader(1896), button(/#1859/)), say: "The problem, what it cost, and the issue it names, with no run" },
  { press: explanation("How to Test"), say: "No run is kept for this one yet, so it runs on choice" },
  { look: inside(reader(1896), role("note", /link mark says reconnecting/)), say: "Steps to try the change on this branch" },
  { press: explanation("How it works"), say: "Back to how it works" },
  { press: tool("Possible bugs"), say: "Possible bugs" },
  { look: inside(results("Possible bugs"), finding("The attempt count is never reset")), say: "A bug, on the change it is in" },
  { press: inside(finding("The attempt count is never reset"), button(/Send to Job/)), say: "Send to Job is there because a Job holds it" },
  { look: text(/Review finding on #1896/), say: "The finding is a message to that Job, which opens on the Cockpit" },
  { press: place("Reviews"), say: "Back to Reviews, where he left it" },
  { look: inside(finding("The attempt count is never reset"), role("img", "Sent to the Job")), say: "Sent once" },
  { press: inside(list, button("Refresh")), say: "Refresh reads the list again, and a commit has been pushed to this one" },
  { look: inside(tool("Possible bugs"), role("img", /Out of date/)), say: "The kept result is marked out of date, naming the commit it ran on" },
  { press: tool("Possible bugs"), say: "Pressed, it still shows the old result" },
  { look: inside(results("Possible bugs"), role("note", "Ran on an older commit")), say: "With a line saying so. Run again is the act that spends a run" },
  { press: tool("Comments"), say: "Request changes on a pull request a Job holds reaches the Job too" },
  { type: "The count has to reset when the socket opens. Cap the jitter too.", into: inside(yours, role("textbox", "Summary")), say: "The review" },
  { press: inside(yours, tab("Request changes")), say: "Request changes" },
  { press: inside(yours, button("Submit review")), say: "Submit records it, and sends it to the Job with the review" },
  { look: inside(yours, role("note", /Sent to Fix the reconnect wait/)), say: "It names the Job it went to" },
];

export const theNewTool: Step[] = [
  { look: tool("Migration safety"), say: "This repository defines a tool of its own, shared with everyone on it" },
  { hover: inside(tool("Test gaps"), role("img", /Replaces the repository's Test gaps/)), say: "His own tool replaces a repository's or Armada's of the same name, so his Test gaps stands in for the repository's" },
  { press: tool("Add a tool"), say: "Add a tool" },
  { type: "Docs drift", into: inside(dialog("Add a tool"), role("textbox", "Name")), say: "A name" },
  { type: "A change to behaviour whose page under docs/concepts still says the old thing.", into: inside(dialog("Add a tool"), role("textbox", "What it checks")), say: "What it checks, as a prompt" },
  { press: inside(dialog("Add a tool"), role("radio", "Repository")), say: "Save it to the repository, where everyone on it has it, or to his Kit, where it follows him" },
  { press: inside(dialog("Add a tool"), button("Save")), say: "Fleet writes the file, in the repository's review-tools folder" },
  { look: tool("Docs drift"), say: "Its tile carries the repository's glyph" },
  { press: tool("Add a tool"), say: "The same sheet adds an explanation" },
  { press: inside(dialog("Add a tool"), role("radio", "Explanation")), say: "Kind: a tool finds things, an explanation tells him one thing about a pull request" },
  { type: "What it changes for support", into: inside(dialog("Add a tool"), role("textbox", "Name")), say: "A name" },
  { type: "What a person answering customers would be asked about after this, and what to tell them.", into: inside(dialog("Add a tool"), role("textbox", "What it explains")), say: "What it explains, as a prompt" },
  { press: inside(dialog("Add a tool"), role("radio", "Repository")), say: "Saved to the repository, like a tool" },
  { press: inside(dialog("Add a tool"), button("Save")), say: "Written the same way" },
  { press: inside(reader(1896), tab("Overview")), say: "Back to the overview" },
  { look: explanation("What it changes for support"), say: "It is one more quick action, carrying the repository's glyph" },
];
