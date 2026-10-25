// Every Job opens on the same start card, with its first message in the Message box to edit before Start. A Job dispatched, a Job another
// Job's agent started, and a Job started from a note on the annotation layer. Over `job-start-card-everywhere`.

import { button, deckRow, inside, place, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const card = role("region", "Start", { exact: true });
const message = inside(card, role("textbox", "Message"));
const start = inside(card, role("button", "Start", { exact: true }));
const thread = region("Thread");
const composer = role("textbox", "Message");
const opened = (title: string) => inside(thread, role("button", title, { exact: true }));
const bar = role("status");

const jobStartCardEverywhere = walk("job-start-card-everywhere", [
  { key: "n", on: jobs, say: "n opens Dispatch" },
  { type: "Add a retry cap to the store client", into: request, say: "Words for a Job" },
  { press: inside(frame, role("radio", "Repository armada", { exact: true })), say: "The repository it runs in" },
  { press: inside(frame, role("button", "Start", { exact: true })), say: "Send. The Job opens on its start card and no agent moves" },
  { type: "Add a retry cap to the store client and stop at 3 attempts", into: message, say: "The Message box holds the words that will be sent, and takes an edit" },
  { press: inside(card, role("radio", "Open PR only", { exact: true })), say: "How it lands" },
  { press: start, say: "Start sends the box as it stands" },
  { look: inside(thread, text(/stop at 3 attempts/)), say: "The edited message is the first one in the thread" },

  { press: deckRow("Debug the retry cap"), say: "A Job whose agent can start another" },
  { type: "Start a session to look into the settings loader ignoring a cap set for one repository\n", into: composer, say: "He asks its agent to start a Job" },
  { press: opened("Settings loader and the cap"), say: "The agent started one. He opens it" },
  { look: message, say: "Under Started by, the same card, with the agent's message in the box" },
  { press: start, say: "Start" },

  { press: place("Cockpit"), say: "Back to the list" },
  { press: inside(bar, button("Start job")), say: "On the annotation layer, Start job on a note" },
  { look: message, say: "The same card. The box holds the note as the agent will read it: his words, then where he pointed" },
  { press: start, say: "Start" },
  { look: inside(thread, text(/Pointed at:/)), say: "The note is the first message in the thread" },
]);

export { jobStartCardEverywhere as "job-start-card-everywhere" };
