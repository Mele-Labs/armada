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

export const toTheList: Step[] = [
  { look: list, say: "The pull requests waiting on him, fetched as the view opened" },
  { hover: inside(list, button("Refresh")), say: "Refresh names when the list was last fetched. Nothing is watched in between" },
  { look: inside(list, tab("Review requested")), say: "Review requested: pull requests someone asked him to review" },
  { press: inside(list, tab("Mine")), say: "Mine: pull requests his own agents opened" },
  { press: inside(list, tab("Review requested")), say: "Back to review requested" },
];

/** A teammate's pull request: nobody of his wrote it, so an agent explains it when he presses. */
export const theExplanation: Step[] = [
  { press: inside(list, button(/#1899/)), say: "Open one a teammate asked for" },
  { look: tool("Risks"), say: "The review tools are tiles in the Flight control. They wait until the pull request is explained" },
  { press: inside(reader(1899), button("Explain")), say: "Nobody of his wrote this one, so an agent explains it, once, when he presses" },
  { look: inside(reader(1899), region("What this does")), say: "What it does, in a few plain sentences" },
  { hover: inside(reader(1899), role("img", "Explained by an agent when you pressed Explain")), say: "Where the explanation came from" },
  { look: theme("Fleet keeps the last read"), say: "Its changes by what they make happen, with the order they were made in" },
  { look: inside(theme("Fleet keeps the last read"), change("Hold the last read for a minute")), say: "Each change says why it was made" },
  { press: inside(change("Hold the last read for a minute"), button("Show change")), say: "The hunk is folded until he opens it" },
  { look: inside(change("Hold the last read for a minute"), thread("Hold the last read for a minute", "devon")), say: "A thread already on the line sits there" },
];

export const theTools: Step[] = [
  { hover: inside(tool("Risks"), role("img", "Armada's tool")), say: "A tool shows where it is defined. This one is Armada's" },
  { press: tool("Risks"), say: "Risks runs for this pull request only when pressed" },
  { look: inside(results("Risks"), finding("A second watch loses the first interval")), say: "Each result is pinned to the change it is about" },
  { look: inside(results("Risks"), theme("Bridge asks once a minute")), say: "Under the theme, then the change" },
  { press: inside(results("Risks"), button("Back to the explanation")), say: "Back to the explanation" },
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
  { press: inside(list, tab("Mine")), say: "Mine" },
  { press: inside(list, button(/#1896/)), say: "One his Session holds" },
  { look: inside(reader(1896), button("Session: Fix the reconnect wait")), say: "The Session that holds it is in the head" },
  { look: inside(reader(1896), region("What this does")), say: "The explanation is already there" },
  { hover: inside(reader(1896), role("img", "Written by the Session that made this pull request")), say: "The Session that wrote it knows why, so nothing is run to get it" },
  { press: tool("Possible bugs"), say: "Possible bugs" },
  { look: inside(results("Possible bugs"), finding("The attempt count is never reset")), say: "A bug, on the change it is in" },
  { press: inside(finding("The attempt count is never reset"), button(/Send to Session/)), say: "Send to Session is there because a Session holds it" },
  { look: inside(finding("The attempt count is never reset"), role("img", "Sent to the Session")), say: "Sent once" },
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
