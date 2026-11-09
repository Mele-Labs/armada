// What the review walks share: everything from the list on. The open pull request is drawn in panes, and each explanation and
// tool opens in a tab of the tools pane, from its + or by its number. Not under `walks/`, where every file is a walk.

import { button, dialog, inside, place, region, role, tab, text } from "./walk";
import type { Step } from "./walk";

const list = region("Pull requests");
const yours = region("Your review");
const reader = (number: number) => region(`Pull request #${number}`);
/** The Review tab's pane, where his review is written. */
const review = (number: number) => inside(reader(number), region("Pane: Review"));
const theme = (title: string) => region(`Theme: ${title}`);
const change = (title: string) => role("article", `Change: ${title}`);
const thread = (title: string, by: string) => role("article", `Thread on ${title} by ${by}`);
const finding = (title: string) => role("article", `Finding: ${title}`);
const results = (name: string) => region(`${name} results`);
const asked = (words: RegExp) =>
  inside(inside(region("Questions asked"), role("list", "Asked")), role("listitem", words));
/** The pane the tools open in, named by the tab in front of it, which is Comments before any opened. */
const toolsPane = (number: number, front?: string) =>
  inside(reader(number), region(`Pane: ${front ?? "Comments"}`));
const plus = (number: number, front?: string) =>
  inside(toolsPane(number, front), button("Add tab"));
/** A tool or explanation by its number, as + lists them: the four explanations, then Armada's tools, then the repository's and the Kit's. */
const NUMBER = {
  "How it works": "1",
  "User Impact": "2",
  "How to Test": "3",
  Why: "4",
  Risks: "5",
  "Possible bugs": "6",
} as const;
const byKey = (number: number, name: keyof typeof NUMBER, say: string): Step => ({
  key: NUMBER[name],
  on: reader(number),
  say,
});

const pick = (name: string | RegExp) => role("menuitem", name);

const lists = inside(list, button("List"));

export const toTheList: Step[] = [
  {
    look: list,
    say: "The pull requests waiting on him, fetched as the view opened",
  },
  {
    hover: inside(list, button("Refresh")),
    say: "Refresh names when the list was last fetched. Nothing is watched in between",
  },
  {
    press: lists,
    say: "One dropdown chooses the list. He can be asked to review in two ways, and both are here",
  },
  {
    press: pick("Requested for Review"),
    say: "Requested for Review: someone asked him by name",
  },
  {
    press: lists,
    say: "The dropdown again",
  },
  {
    press: pick("Team"),
    say: "Team: a team he is on was asked",
  },
  {
    look: inside(list, button(/#1899/)),
    say: "A pull request can be on two lists. This one asked him by name and asked his team",
  },
  {
    press: lists,
    say: "The dropdown again",
  },
  {
    press: pick("Assigned"),
    say: "Assigned: pull requests the forge assigned to him",
  },
  {
    look: inside(list, button(/#1899/)),
    say: "The same pull request, here too",
  },
  {
    press: lists,
    say: "The dropdown again",
  },
  {
    press: pick("My PRs"),
    say: "My PRs: pull requests his own agents opened",
  },
  {
    press: lists,
    say: "Natural Search is the last entry. It is not a box on the list",
  },
  {
    press: pick("Natural Search"),
    say: "Choosing it puts the box in place of the list",
  },
  {
    type: "PRs that touch files I've touched",
    into: inside(list, role("textbox", "Natural Search")),
    say: "What he wants, in his own words",
  },
  {
    press: inside(list, button("Search")),
    say: "An agent looks only when he sends, because it costs",
  },
  {
    look: dialog("Natural Search"),
    say: "It spends tokens, so it asks first. Every act that spends asks the same way, with the same box",
  },
  {
    press: inside(dialog("Natural Search"), button("Run")),
    say: "Run",
  },
  {
    look: inside(list, role("img", "Looking")),
    say: "Running, in the list",
  },
  {
    press: lists,
    say: "The find is saved in the dropdown under his words",
  },
  {
    look: pick(/files I've touched/),
    say: "Run again and Remove are on the list's head",
  },
  {
    press: pick("Requested for Review"),
    say: "Back to Requested for Review",
  },
];

/** A teammate's pull request: its description comes first, and each explanation is a tab that runs once. */
export const theExplanation: Step[] = [
  {
    press: lists,
    say: "A longer one first, from the team's list",
  },
  {
    press: pick("Team"),
    say: "Team",
  },
  {
    key: "l",
    on: list,
    say: "L goes to the list, as it does on the Cockpit. Everything from here is the keyboard",
  },
  {
    key: "ArrowDown",
    on: inside(list, button(/#1899/)),
    say: "↓ moves down the list. J does too",
  },
  {
    key: "ArrowDown",
    on: inside(list, button(/#1903/)),
    say: "Down again",
  },
  {
    key: "Enter",
    on: inside(list, button(/#1907/)),
    say: "↵ opens a pull request with a long description",
  },
  {
    look: toolsPane(1907),
    say: "It opens in panes. The explanations and the tools open in tabs of the pane on the right",
  },
  {
    look: inside(reader(1907), region("Description")),
    say: "Overview is in front, and its description is open",
  },
  {
    key: "Shift+Q",
    on: list,
    say: "⇧Q opens the tools pane's +, from any part",
  },
  {
    look: role("menuitem", "How it works"),
    say: "The four explanations first, then the tools, each with its number",
  },
  {
    key: "Escape",
    on: role("menuitem", "How it works"),
    say: "Esc puts it away",
  },
  byKey(1907, "How to Test", "3 is How to Test. No kept result, so its tab waits for Run"),
  {
    look: inside(reader(1907), text("Running this uses tokens.")),
    say: "The line beside Run says so",
  },
  {
    press: inside(reader(1907), role("checkbox", "Don't ask again")),
    say: "Don't ask again turns the question off for every act that spends tokens. It is a setting on this machine",
  },
  {
    press: inside(reader(1907), button("Run")),
    say: "Run",
  },
  {
    look: inside(reader(1907), region("How to Test")),
    say: "The explanation fills its own tab",
  },
  {
    hover: inside(
      inside(reader(1907), role("listitem", "Step 1")),
      role("img", "From the description"),
    ),
    say: "How to Test follows the author's own directions first. Each step says where it came from",
  },
  {
    hover: inside(
      inside(reader(1907), role("listitem", "Step 3")),
      button(/crates\/fleet\/README/),
    ),
    say: "The description does not cover this area, so the step comes from the nearest README. Its path opens it",
  },
  {
    key: "Shift+D",
    on: reader(1907),
    say: "⇧D folds the description, beside it in Overview",
  },
  {
    look: inside(reader(1907), button("Show the description")),
    say: "Folded into one row",
  },
  {
    key: "Shift+D",
    on: reader(1907),
    say: "⇧D opens it again",
  },
  {
    look: inside(reader(1907), region("Description")),
    say: "Open",
  },
  byKey(
    1907,
    "User Impact",
    "2 is User Impact. The question is off now, so it runs without asking",
  ),
  {
    look: inside(reader(1907), role("listitem", "Restarting Fleet")),
    say: "What a user sees differently, before and after, in a tab beside How to Test",
  },
  {
    key: "Escape",
    on: reader(1907),
    say: "Esc backs out a level: from the reader to the list",
  },
  {
    key: "Shift+L",
    on: list,
    say: "⇧L opens the lists",
  },
  {
    look: pick("Requested for Review"),
    say: "The menu answers its own arrows and ↵. Natural Search is /",
  },
  {
    press: pick("Requested for Review"),
    say: "Requested for Review",
  },
  {
    press: inside(list, button(/#1899/)),
    say: "Open one a teammate asked for",
  },
  {
    look: inside(reader(1899), region("Description")),
    say: "The author's own description",
  },
  {
    look: toolsPane(1899),
    say: "Nothing has run on this one, so the pane on the right holds Comments alone",
  },
  {
    press: plus(1899),
    say: "Its + lists every explanation and tool",
  },
  {
    press: pick("How to Test"),
    say: "How to Test. An agent reads the pull request, once",
  },
  {
    look: inside(reader(1899), region("How to Test")),
    say: "Numbered steps, each with what to look for",
  },
  {
    look: inside(reader(1899), role("note", /One provider call a minute/)),
    say: "What to look for sits under the step",
  },
  {
    hover: inside(reader(1899), role("img", "Explained by an agent when you chose it")),
    say: "Where the explanation came from",
  },
  {
    look: inside(inside(reader(1899), region("How to Test")), role("img", /^Kept/)),
    say: "It is kept for the commit it read, as a tool's result is",
  },
  byKey(1899, "User Impact", "2: what it changes for you, as someone who uses it"),
  {
    look: inside(reader(1899), role("listitem", "The plan meters in the title bar")),
    say: "What they see or can do differently, before and after",
  },
  {
    press: inside(reader(1899), tab("How to Test")),
    say: "Back to how to test it, by its tab",
  },
  {
    look: inside(reader(1899), role("list", "Steps")),
    say: "Shown at once. Nothing runs and nothing is spent",
  },
  byKey(1899, "Why", "4: why it was needed"),
  {
    look: inside(reader(1899), button(/#1871/)),
    say: "The problem, what it cost, and the issue the pull request names",
  },
  {
    hover: inside(inside(reader(1899), region("Why")), role("img", "Armada's tool")),
    say: "Explanations are defined where tools are: Armada's four, the Kit, and the repository",
  },
  {
    look: inside(reader(1899), role("textbox", "Ask about this PR")),
    say: "Ask is not a tool. It is a tab of its own, docked under the panes while a pull request is open",
  },
  {
    look: inside(reader(1899), role("list", "Asked")),
    say: "A question asked before is kept in a small panel above the description, under his words",
  },
  {
    key: "Shift+A",
    on: reader(1899),
    say: "⇧A focuses the composer from any part. Its key is beside it",
  },
  {
    type: "Does this change what happens when gh is signed out?",
    into: inside(reader(1899), role("textbox", "Ask about this PR")),
    say: "Whatever he wants to know about this one",
  },
  {
    key: "Enter",
    on: inside(reader(1899), role("textbox", "Ask about this PR")),
    say: "↵ sends and ⇧↵ breaks the line. An agent answers only when he sends, because it costs",
  },
  {
    look: inside(reader(1899), role("img", "Answering")),
    say: "Running, in place",
  },
  {
    look: inside(asked(/Does this change what happens/), role("img", /^Kept/)),
    say: "The question joins the panel, and its answer is kept for the commit it read",
  },
  {
    hover: inside(asked(/Does this change what happens/), button("Remove this question")),
    say: "Run again asks it once more. Remove drops it",
  },
  {
    key: "o",
    on: inside(asked(/What happens to the cached read/), button(/What happens to the cached read/)),
    say: "O shows an earlier question's answer, as ↵ does, and at once",
  },
  {
    look: region("Answer"),
    say: "The answer, under the description, with nothing run",
  },
  byKey(1899, "Risks", "5 is Risks, the first of the review tools. They work on any pull request"),
  {
    look: inside(results("Risks"), finding("A second watch loses the first interval")),
    say: "Not explained yet, so a result is pinned to a file and its lines",
  },
  {
    hover: inside(results("Risks"), role("img", "Armada's tool")),
    say: "A tool shows where it is defined. This one is Armada's",
  },
  {
    look: inside(results("Risks"), role("img", /^Kept/)),
    say: "A finished result is kept: its head says when it ran and on which commit",
  },
  {
    press: inside(reader(1899), tab("Why")),
    say: "Another tab",
  },
  byKey(1899, "Risks", "5 again brings Risks forward. Nothing runs and nothing is spent"),
  {
    look: inside(results("Risks"), finding("A second watch loses the first interval")),
    say: "The same result, kept",
  },
  byKey(
    1899,
    "How it works",
    "1 is How it works, the themes view. An agent reads it the first time, like the rest",
  ),
  {
    look: inside(reader(1899), region("How it works")),
    say: "What it does in a few plain sentences",
  },
  {
    look: theme("Fleet keeps the last read"),
    say: "Its changes by what they make happen, with the order they were made in",
  },
  {
    look: inside(theme("Fleet keeps the last read"), change("Hold the last read for a minute")),
    say: "Each change says why it was made",
  },
  {
    press: inside(change("Hold the last read for a minute"), button("Show diff")),
    say: "The hunk is folded until he opens it",
  },
  {
    look: inside(
      change("Hold the last read for a minute"),
      thread("Hold the last read for a minute", "devon"),
    ),
    say: "A thread already on the line sits there",
  },
];

export const theTools: Step[] = [
  byKey(1899, "Risks", "Risks again, now that it is explained"),
  {
    look: inside(results("Risks"), theme("Bridge asks once a minute")),
    say: "The same result sits under its theme and then its change",
  },
  byKey(1899, "Possible bugs", "6 is Possible bugs"),
  {
    look: inside(results("Possible bugs"), finding("A reset inside the minute is served late")),
    say: "One bug, on the change that serves the read",
  },
  {
    press: inside(
      reader(1899),
      tab("Review", {
        exact: true,
      }),
    ),
    say: "Review is a tab of the main pane: a file at a time, the findings beside their lines, and his review",
  },
  {
    press: inside(review(1899), button(/^Comment on line \d+: let mut held = self\.held\.lock/)),
    say: "A comment on a line, from the gutter",
  },
  {
    type: "Take the lock after the call, so a slow read does not hold it.",
    into: inside(review(1899), role("textbox", "Comment")),
    say: "Written under the line",
  },
  {
    press: inside(review(1899), button("Start a review")),
    say: "It starts the pending review. Nothing is posted yet",
  },
  {
    press: inside(review(1899), button("Review changes")),
    say: "Review changes opens his review under the file",
  },
  {
    look: inside(yours, role("list", "Pending comments")),
    say: "Pending comments wait here to be posted together, each one press from where it is",
  },
  {
    type: "The lock should be taken after the call. Fix that and the reset case.",
    into: inside(yours, role("textbox", "Summary")),
    say: "The summary",
  },
  {
    press: inside(yours, tab("Request changes")),
    say: "Request changes",
  },
  {
    press: inside(yours, button("Submit review")),
    say: "Submit posts the pending comments and the verdict together",
  },
  {
    look: inside(yours, role("alert")),
    say: "Neither tool's findings are marked, so it says so once",
  },
  {
    press: inside(yours, button("Submit anyway")),
    say: "Submit anyway sends it",
  },
  {
    look: inside(yours, role("list", "Submitted reviews")),
    say: "Recorded here",
  },
  {
    look: inside(list, role("img", "Changes requested")),
    say: "And on the row",
  },
  {
    press: inside(list, button(/Bump the manifest icons/)),
    say: "A pull request a tool has nothing to say about",
  },
  {
    key: "5",
    on: region(/^Pull request #/),
    say: "Risks, on this one",
  },
  {
    look: inside(results("Risks"), role("img", "Nothing found")),
    say: "Nothing found is a small picture in the tool's colour, with what was checked under it",
  },
];

/** One of his Sessions holds the pull request, so it knows why and the explanation is kept from the start. */
export const theSessionsOwn: Step[] = [
  {
    press: lists,
    say: "The dropdown again",
  },
  {
    press: pick("My PRs"),
    say: "My PRs",
  },
  {
    press: inside(list, button(/#1896/)),
    say: "One his Job holds",
  },
  {
    look: inside(reader(1896), button("Job: Fix the reconnect wait")),
    say: "The Job that holds it is in the head",
  },
  byKey(1896, "How it works", "1: How it works"),
  {
    look: inside(reader(1896), region("How it works")),
    say: "Already there, since the Job's agent explained it",
  },
  {
    hover: inside(inside(reader(1896), region("How it works")), role("img", /^Kept/)),
    say: "Kept for the commit it read, so nothing is run to get it",
  },
  byKey(1896, "Why", "4: Why was kept the same way"),
  {
    look: inside(reader(1896), button(/#1859/)),
    say: "The problem, what it cost, and the issue it names, with no run",
  },
  byKey(1896, "How to Test", "3: No run is kept for How to Test yet, so it runs on opening"),
  {
    look: inside(reader(1896), role("note", /link mark says reconnecting/)),
    say: "Steps to try the change on this branch",
  },
  {
    press: inside(reader(1896), tab("How it works")),
    say: "Back to how it works, by its tab",
  },
  byKey(1896, "Possible bugs", "6: Possible bugs"),
  {
    look: inside(results("Possible bugs"), finding("The attempt count is never reset")),
    say: "A bug, on the change it is in",
  },
  {
    press: inside(finding("The attempt count is never reset"), button(/Send to Job/)),
    say: "Send to Job is there because a Job holds it",
  },
  {
    look: text(/Review finding on #1896/),
    say: "The finding is a message to that Job, which opens on the Cockpit",
  },
  {
    press: place("Reviews"),
    say: "Back to Reviews, where he left it",
  },
  {
    look: inside(finding("The attempt count is never reset"), role("img", "Sent to the Job")),
    say: "Sent once",
  },
  {
    press: inside(list, button("Refresh")),
    say: "Refresh reads the list again, and a commit has been pushed to this one",
  },
  {
    look: inside(results("Possible bugs"), role("img", /Out of date/)),
    say: "The kept result is marked out of date, naming the commit it ran on",
  },
  {
    look: inside(results("Possible bugs"), role("note", "Ran on an older commit")),
    say: "With a line saying so. Run again is the act that spends a run",
  },
  {
    press: inside(
      reader(1896),
      tab("Review", {
        exact: true,
      }),
    ),
    say: "Request changes on a pull request a Job holds reaches the Job too",
  },
  {
    press: inside(review(1896), button("Review changes")),
    say: "His review, under the file",
  },
  {
    type: "The count has to reset when the socket opens. Cap the jitter too.",
    into: inside(yours, role("textbox", "Summary")),
    say: "The review",
  },
  {
    press: inside(yours, tab("Request changes")),
    say: "Request changes",
  },
  {
    press: inside(yours, button("Submit review")),
    say: "Submit records it, and sends it to the Job with the review",
  },
  {
    press: inside(yours, button("Submit anyway")),
    say: "Past the line about the finding not marked",
  },
  {
    look: inside(yours, role("note", /Sent to Fix the reconnect wait/)),
    say: "It names the Job it went to",
  },
];

export const theNewTool: Step[] = [
  {
    press: plus(1896, "Possible bugs"),
    say: "The tools pane's + again",
  },
  {
    look: pick("Migration safety"),
    say: "This repository defines a tool of its own, shared with everyone on it",
  },
  {
    hover: pick("Test gaps"),
    say: "His own tool replaces a repository's or Armada's of the same name, so his Test gaps stands in for the repository's",
  },
  {
    press: pick("Add a tool"),
    say: "Add a tool, the last entry",
  },
  {
    type: "Docs drift",
    into: inside(dialog("Add a tool"), role("textbox", "Name")),
    say: "A name",
  },
  {
    type: "A change to behaviour whose page under docs/concepts still says the old thing.",
    into: inside(dialog("Add a tool"), role("textbox", "What it checks")),
    say: "What it checks, as a prompt",
  },
  {
    press: inside(dialog("Add a tool"), role("radio", "Repository")),
    say: "Save it to the repository, where everyone on it has it, or to his Kit, where it follows him",
  },
  {
    press: inside(dialog("Add a tool"), button("Save")),
    say: "Fleet writes the file, in the repository's review-tools folder",
  },
  {
    press: plus(1896, "Possible bugs"),
    say: "+ again",
  },
  {
    look: pick("Docs drift"),
    say: "The new tool is listed with the rest",
  },
  {
    press: pick("Add a tool"),
    say: "The same sheet adds an explanation",
  },
  {
    press: inside(dialog("Add a tool"), role("radio", "Explanation")),
    say: "Kind: a tool finds things, an explanation tells him one thing about a pull request",
  },
  {
    type: "What it changes for support",
    into: inside(dialog("Add a tool"), role("textbox", "Name")),
    say: "A name",
  },
  {
    type: "What a person answering customers would be asked about after this, and what to tell them.",
    into: inside(dialog("Add a tool"), role("textbox", "What it explains")),
    say: "What it explains, as a prompt",
  },
  {
    press: inside(dialog("Add a tool"), role("radio", "Repository")),
    say: "Saved to the repository, like a tool",
  },
  {
    press: inside(dialog("Add a tool"), button("Save")),
    say: "Written the same way",
  },
  {
    press: plus(1896, "Possible bugs"),
    say: "+ once more",
  },
  {
    look: pick("What it changes for support"),
    say: "It is one more entry, among the explanations",
  },
  {
    key: "Escape",
    on: pick("What it changes for support"),
    say: "Esc puts it away",
  },
];
