// What the two review walks share: everything from the list on. They differ only in how the review surface is reached
// (the Cockpit's Flight control, or a tab of its own) and in what they do on leaving it. Not under `walks/`, where
// every file is a walk.

import { button, dialog, inside, region, role, tab } from "./walk";
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

const pick = (name: string | RegExp) => role("menuitem", name);
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
  { press: inside(list, button("Search")), say: "A Drone looks only when he sends, because it costs" },
  { look: inside(list, role("img", "Looking")), say: "Running, in the list" },
  { look: inside(list, role("note", /which you changed on 14 Oct/)), say: "What it found, each with one line on why it matched" },
  { press: lists, say: "The find is saved in the dropdown under his words" },
  { look: pick(/files I've touched/), say: "Run again and Remove are on the list's head" },
  { press: pick("Requested for Review"), say: "Back to Requested for Review" },
];

/** A teammate's pull request: its description comes first, and Explain is the optional deeper step. */
export const theExplanation: Step[] = [
  { press: inside(list, button(/#1899/)), say: "Open one a teammate asked for" },
  { look: inside(reader(1899), region("Description")), say: "The author's own description comes first" },
  { look: tool("Risks"), say: "The review tools are tiles in the Flight control, and they work on any pull request" },
  { hover: inside(tool("Risks"), role("img", "Armada's tool")), say: "A tool shows where it is defined. This one is Armada's" },
  { press: tool("Risks"), say: "Risks runs for this pull request only when pressed" },
  { look: inside(results("Risks"), finding("A second watch loses the first interval")), say: "Not explained yet, so a result is pinned to a file and its lines" },
  { look: inside(tool("Risks"), role("img", /^Kept/)), say: "A finished result is kept: the tile says when it ran and on which commit" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
  { press: tool("Risks"), say: "Pressed again, it shows at once. Nothing runs and nothing is spent" },
  { look: inside(results("Risks"), finding("A second watch loses the first interval")), say: "The same result, kept" },
  { press: inside(results("Risks"), button("Back to the overview")), say: "Back to the overview" },
  { press: inside(reader(1899), button("Explain")), say: "Explain is optional. An agent adds what it does and its themes, once, when he presses" },
  { look: inside(reader(1899), region("What this does")), say: "What it does, in a few plain sentences, under the description" },
  { hover: inside(reader(1899), role("img", "Explained by an agent when you pressed Explain")), say: "Where the explanation came from" },
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
];

/** One of his Sessions holds the pull request, so it knows why and the explanation is there at open. */
export const theSessionsOwn: Step[] = [
  { press: lists, say: "The dropdown again" },
  { press: pick("My PRs"), say: "My PRs" },
  { press: inside(list, button(/#1896/)), say: "One his Session holds" },
  { look: inside(reader(1896), button("Session: Fix the reconnect wait")), say: "The Session that holds it is in the head" },
  { look: inside(reader(1896), region("Description")), say: "The description is first here too" },
  { look: inside(reader(1896), region("What this does")), say: "The Session's explanation is already under it" },
  { hover: inside(reader(1896), role("img", "Written by the Session that made this pull request")), say: "The Session that wrote it knows why, so nothing is run to get it" },
  { press: tool("Possible bugs"), say: "Possible bugs" },
  { look: inside(results("Possible bugs"), finding("The attempt count is never reset")), say: "A bug, on the change it is in" },
  { press: inside(finding("The attempt count is never reset"), button(/Send to Session/)), say: "Send to Session is there because a Session holds it" },
  { look: inside(finding("The attempt count is never reset"), role("img", "Sent to the Session")), say: "Sent once" },
  { press: inside(list, button("Refresh")), say: "Refresh reads the list again, and a commit has been pushed to this one" },
  { look: inside(tool("Possible bugs"), role("img", /Out of date/)), say: "The kept result is marked out of date, naming the commit it ran on" },
  { press: tool("Possible bugs"), say: "Pressed, it still shows the old result" },
  { look: inside(results("Possible bugs"), role("note", "Ran on an older commit")), say: "With a line saying so. Run again is the act that spends a run" },
  { press: tool("Comments"), say: "Request changes on a pull request a Session holds reaches the Session too" },
  { type: "The count has to reset when the socket opens. Cap the jitter too.", into: inside(yours, role("textbox", "Summary")), say: "The review" },
  { press: inside(yours, tab("Request changes")), say: "Request changes" },
  { press: inside(yours, button("Submit review")), say: "Submit records it, and sends it to the Session with the review" },
  { look: inside(yours, role("note", /Sent to Fix the reconnect wait/)), say: "It names the Session it went to" },
];

export const theNewTool: Step[] = [
  { look: tool("Migration safety"), say: "This repository defines a tool of its own, shared with everyone on it" },
  { hover: inside(tool("Test gaps"), role("img", /Replaces the Kit's Test gaps/)), say: "A repository's tool replaces a Kit or Armada tool of the same name" },
  { press: tool("Add a tool"), say: "Add a tool" },
  { type: "Docs drift", into: inside(dialog("Add a tool"), role("textbox", "Name")), say: "A name" },
  { type: "A change to behaviour whose page under docs/concepts still says the old thing.", into: inside(dialog("Add a tool"), role("textbox", "What it checks")), say: "What it checks, as a prompt" },
  { press: inside(dialog("Add a tool"), role("radio", "Repository")), say: "Save it to the repository, where everyone on it has it, or to his Kit, where it follows him" },
  { press: inside(dialog("Add a tool"), button("Save")), say: "It is recorded and not written to a file here" },
  { look: tool("Docs drift"), say: "Its tile carries the repository's glyph" },
];
