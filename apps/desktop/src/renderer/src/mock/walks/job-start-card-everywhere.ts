// Every Job opens like any other Job, held: its title in the header, its first message in the composer, its Workflow and Delivery in the strip
// on the composer already set, and Start where Send is. A Job
// dispatched, a Job another Job's agent started, and a Job started from a note on the annotation layer. Over `job-start-card-everywhere`.

import { button, deckRow, inside, place, region, role, text, walk } from "../walk";

const jobs = region("Jobs", { exact: true });
const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const composer = role("textbox", "Message");
const delivery = role("combobox", "Delivery");
const start = role("button", "Start", { exact: true });
const thread = region("Thread");
const opened = (title: string) => inside(thread, role("button", title, { exact: true }));
const bar = role("status");

const jobStartCardEverywhere = walk("job-start-card-everywhere", [
  { key: "n", on: jobs, say: "n opens Dispatch" },
  { type: "Add a retry cap to the store client", into: request, say: "Words for a Job" },
  { press: inside(frame, role("radio", "Repository armada", { exact: true })), say: "The repository it runs in" },
  { press: inside(frame, role("button", "Start", { exact: true })), say: "Send. The Job opens and no agent moves" },
  { look: inside(composer, text(/Add a retry cap to the store client/)), say: "Its words are in the composer" },
  { hover: role("img", "Picked by the proposer"), say: "The proposer read the request and picked the Workflow" },
  { hover: role("img", /^Your usual/), say: "Delivery is already his usual" },
  { type: "Add a retry cap to the store client and stop at 3 attempts", into: composer, say: "He edits the words" },
  { type: "open_pr", into: delivery, say: "He changes Delivery" },
  { press: start, say: "Send reads Start" },
  { look: inside(thread, text(/stop at 3 attempts/)), say: "The edited message is the first one in the thread" },

  { press: deckRow("Debug the retry cap"), say: "A Job whose agent can start another" },
  { type: "Start a session to look into the settings loader ignoring a cap set for one repository\n", into: composer, say: "He asks its agent to start a Job" },
  { press: opened("Settings loader and the cap"), say: "The agent started one. He opens it" },
  { look: inside(composer, text(/Read crates\/settings\/src\/loader\.rs/)), say: "Under Started by, the agent's message is in the composer, with Workflow and Delivery already set" },
  { press: start, say: "Start" },

  { press: place("Cockpit"), say: "Back to the list" },
  { key: "Alt+Meta+a", on: jobs, say: "⌥⌘A turns the annotation layer on over a note already pinned" },
  { press: inside(bar, button("Start job")), say: "On the annotation layer, Start job on a note" },
  { look: inside(composer, text(/Pointed at:/)), say: "The composer holds the note as the agent will read it: his words, then where he pointed. Workflow and Delivery are set" },
  { key: "Alt+Meta+a", on: jobs, say: "⌥⌘A turns the layer off, so the press on Start is a press" },
  { press: start, say: "Start" },
  { look: inside(thread, text(/Pointed at:/)), say: "The note is the first message in the thread" },
]);

export { jobStartCardEverywhere as "job-start-card-everywhere" };
